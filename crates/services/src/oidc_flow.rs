#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The browser half of single sign-on: the authorization-code flow.
//!
//! Task 7's middleware lets a caller who *already holds* a provider token
//! authenticate. This is what gets a person in a browser one. It runs
//! server-side, so `RDataCore` ends up issuing its own access and refresh
//! tokens — the same ones `login` issues — and refresh, logout and
//! revoke-all keep working with no single-sign-on branches anywhere.
//!
//! Three things here are security-critical and none of them are optional.
//!
//! **PKCE is mandatory**, not negotiated. An authorization code intercepted
//! on its way back is useless without the verifier, which never leaves this
//! server.
//!
//! **`state` is single-use, and the claim is atomic.** A read followed by a
//! delete leaves a window in which two concurrent callbacks both succeed.
//! Claiming through the cache's atomic counter closes it: the first request
//! to claim a state gets `1`, and every later one is refused.
//!
//! **`return_to` is a path inside this application or it is nothing.** An
//! unvalidated one makes the login endpoint an open redirect, and an open
//! redirect on a login endpoint is a phishing primitive.

use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::oidc::keys::OidcClaims;
use r_data_core_core::oidc::{is_local_redirect_path, OidcConfig};

use crate::oidc_discovery;
use crate::oidc_runtime::{OidcAuthError, OidcRuntime};

/// How long a started sign-in may sit unfinished.
const STATE_TTL: Duration = Duration::from_secs(300);
/// Cache-key prefix for a pending sign-in.
const STATE_PREFIX: &str = "oidc:state:";
/// Cache-key prefix for the atomic single-use claim on a state.
const CLAIM_PREFIX: &str = "oidc:state-claim:";
/// Bytes of randomness in the `state` value and the PKCE verifier.
const RANDOM_BYTES: usize = 32;

#[derive(Debug, Error)]
pub enum FlowError {
    #[error(
        "the browser login flow is not configured; set RDC_OIDC_CLIENT_ID and \
         RDC_OIDC_REDIRECT_URI to enable it"
    )]
    NotConfigured,
    #[error("the provider's discovery document names no {0}")]
    MissingEndpoint(&'static str),
    #[error("could not reach the identity provider: {0}")]
    Provider(String),
    #[error(
        "this sign-in is not one we started, or it has already been completed. \
         Start again from the sign-in page."
    )]
    UnknownState,
    #[error("the identity provider returned no usable identity token")]
    NoIdToken,
    #[error(transparent)]
    Auth(#[from] OidcAuthError),
}

/// The part of a token-endpoint response this system reads.
#[derive(Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
}

/// A pending sign-in, held between `start` and `callback`.
#[derive(Serialize, Deserialize)]
struct PendingLogin {
    /// The PKCE verifier. Never leaves this server until the code exchange.
    verifier: String,
    /// Where to land afterwards, already validated as a local path.
    return_to: Option<String>,
}

/// Where to send the browser, and the state that identifies the attempt.
pub struct Redirect {
    pub url: String,
    pub state: String,
}

/// Drives the authorization-code flow.
pub struct OidcFlow {
    runtime: Arc<OidcRuntime>,
    cache: Arc<CacheManager>,
    http: reqwest::Client,
}

impl OidcFlow {
    /// # Errors
    /// Returns `FlowError::Provider` if the HTTP client cannot be built.
    pub fn new(runtime: Arc<OidcRuntime>, cache: Arc<CacheManager>) -> Result<Self, FlowError> {
        Ok(Self {
            runtime,
            cache,
            http: oidc_discovery::client().map_err(|e| FlowError::Provider(e.to_string()))?,
        })
    }

    fn config(&self) -> &OidcConfig {
        self.runtime.config()
    }

    /// Whether the operator has configured enough for a browser sign-in.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.config().client_id.is_some() && self.config().redirect_uri.is_some()
    }

    /// Begin a sign-in: mint PKCE and state, and say where to send the browser.
    ///
    /// # Errors
    /// Returns `FlowError` when the flow is not configured or the provider
    /// cannot be reached.
    pub async fn start(&self, return_to: Option<&str>) -> Result<Redirect, FlowError> {
        let (Some(client_id), Some(redirect_uri)) =
            (&self.config().client_id, &self.config().redirect_uri)
        else {
            return Err(FlowError::NotConfigured);
        };

        let discovery = oidc_discovery::fetch(&self.http, &self.config().issuer)
            .await
            .map_err(|e| FlowError::Provider(e.to_string()))?;
        let authorize = discovery
            .authorization_endpoint
            .ok_or(FlowError::MissingEndpoint("authorization_endpoint"))?;

        let verifier = random_token();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_token();

        self.cache
            .set(
                &format!("{STATE_PREFIX}{state}"),
                &PendingLogin {
                    verifier,
                    // Anything that is not a local path is dropped rather than
                    // rejected: a stale bookmark should still log someone in,
                    // just to the default place.
                    return_to: return_to
                        .filter(|r| is_local_redirect_path(r))
                        .map(str::to_string),
                },
                Some(STATE_TTL.as_secs()),
            )
            .await
            .map_err(|e| FlowError::Provider(format!("could not record the sign-in: {e}")))?;

        let query = [
            ("response_type", "code"),
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("scope", "openid profile email"),
            ("state", state.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ];
        let separator = if authorize.contains('?') { '&' } else { '?' };
        let url = format!("{authorize}{separator}{}", encode_query(&query));

        Ok(Redirect { url, state })
    }

    /// Complete a sign-in: claim the state, exchange the code, verify the
    /// identity token, and resolve it to a session.
    ///
    /// # Errors
    /// Returns `FlowError` when the state is unknown or already used, the
    /// provider refuses the exchange, or the identity may not sign in.
    pub async fn complete(
        &self,
        state: &str,
        code: &str,
        session_seconds: u64,
    ) -> Result<(OidcClaims, Option<String>), FlowError> {
        // Atomic, and first: a replay must lose the race rather than be
        // noticed afterwards. A cache that is disabled or unreachable counts
        // as "not claimed", which refuses the sign-in — the safe direction.
        let claim = self
            .cache
            .increment(&format!("{CLAIM_PREFIX}{state}"), STATE_TTL.as_secs())
            .await
            .unwrap_or(0);
        if claim != 1 {
            return Err(FlowError::UnknownState);
        }

        let key = format!("{STATE_PREFIX}{state}");
        let pending: PendingLogin = self
            .cache
            .get(&key)
            .await
            .ok()
            .flatten()
            .ok_or(FlowError::UnknownState)?;
        // Belt and braces: the claim above already made this single-use, but
        // leaving the verifier in the cache for its full TTL serves nothing.
        let _ = self.cache.delete(&key).await;

        let id_token = self.exchange_code(code, &pending.verifier).await?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let verified = self.runtime.verify(&id_token, now).await?;

        // The same refusals as the bearer path: unmapped, deactivated, locked.
        // Reached through the shared tail so it cannot diverge.
        self.runtime
            .session_claims(&verified, session_seconds)
            .await?;

        Ok((verified, pending.return_to))
    }

    /// Trade an authorization code for tokens at the provider.
    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<String, FlowError> {
        let (Some(client_id), Some(redirect_uri)) =
            (&self.config().client_id, &self.config().redirect_uri)
        else {
            return Err(FlowError::NotConfigured);
        };

        let discovery = oidc_discovery::fetch(&self.http, &self.config().issuer)
            .await
            .map_err(|e| FlowError::Provider(e.to_string()))?;
        let token_endpoint = discovery
            .token_endpoint
            .ok_or(FlowError::MissingEndpoint("token_endpoint"))?;

        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", client_id.as_str()),
            ("code_verifier", verifier),
        ];
        if let Some(secret) = &self.config().client_secret {
            form.push(("client_secret", secret.expose()));
        }

        let response = self
            .http
            .post(&token_endpoint)
            .form(&form)
            .send()
            .await
            .map_err(|e| FlowError::Provider(format!("{token_endpoint}: {e}")))?;

        if !response.status().is_success() {
            // The provider's body may echo the code or the secret back; the
            // status is enough to act on and safe to keep.
            return Err(FlowError::Provider(format!(
                "the token endpoint answered {}",
                response.status()
            )));
        }

        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| FlowError::Provider(format!("{token_endpoint}: {e}")))?;

        // The ID token, not the access token: an ID token is always a JWT
        // carrying the claims needed, whereas an access token is opaque at
        // many providers and cannot be validated here at all.
        tokens.id_token.ok_or(FlowError::NoIdToken)
    }

    /// Where to land after a successful sign-in.
    #[must_use]
    pub fn landing_path(&self, return_to: Option<&str>) -> String {
        return_to
            .filter(|r| is_local_redirect_path(r))
            .map_or_else(|| self.config().post_login_path.clone(), str::to_string)
    }
}

/// 32 bytes of randomness, URL-safe.
fn random_token() -> String {
    let mut bytes = [0u8; RANDOM_BYTES];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Percent-encode a query string.
///
/// Hand-rolled rather than pulling in a URL crate for one call: only the
/// characters that must be escaped in a query value are, and everything
/// unreserved is left alone.
fn encode_query(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The OIDC pieces the API layer holds, wired together once at startup.
///
/// One handle rather than two, because the middleware and the login routes
/// must never end up looking at differently-configured halves of the same
/// feature.
pub struct OidcServices {
    runtime: Arc<OidcRuntime>,
    flow: OidcFlow,
}

impl OidcServices {
    /// # Errors
    /// Returns `FlowError::Provider` if the HTTP client cannot be built.
    pub fn new(runtime: Arc<OidcRuntime>, cache: Arc<CacheManager>) -> Result<Self, FlowError> {
        let flow = OidcFlow::new(Arc::clone(&runtime), cache)?;
        Ok(Self { runtime, flow })
    }

    #[must_use]
    pub const fn runtime(&self) -> &Arc<OidcRuntime> {
        &self.runtime
    }

    #[must_use]
    pub const fn flow(&self) -> &OidcFlow {
        &self.flow
    }
}

#[cfg(test)]
mod oidc_flow_tests;
