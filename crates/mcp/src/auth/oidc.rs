#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Turning a validated caller identity into a token this server may use.
//!
//! **The caller's token is never forwarded.** It is presented to `RDataCore`'s
//! exchange endpoint, which answers with a short-lived local access token
//! minted for that same person. Passing a received token on to a downstream
//! service is the pattern the MCP authorization specification names and
//! disallows; it would also require multi-audience tokens, which Keycloak
//! issues easily and Entra and Auth0 largely do not.
//!
//! **There is no standing credential.** With no valid caller this backend
//! produces nothing — it does not reach for a configured key. That is the
//! guarantee the whole permission model rests on: every outbound request
//! carries a token belonging to the human on whose behalf it is acting, so a
//! 403 from `RDataCore` is authoritative and no bug here can exceed what that
//! person may do.
//!
//! **Validation happens once, in the HTTP layer**, which sets `validated` on
//! the caller context. Re-verifying a signature on every outbound call would
//! be waste; trusting an *unvalidated* context would be a hole. So the flag is
//! required, and its absence is an error rather than a prompt to validate here.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

use super::{AuthBackend, AuthError, CallerContext};
use crate::config::Config;

/// Path of `RDataCore`'s token-exchange endpoint.
const EXCHANGE_PATH: &str = "admin/api/v1/auth/oidc/exchange";

/// Fraction of a token's life after which it is re-exchanged.
///
/// Refreshing early rather than at expiry means a call never races the
/// boundary and gets a 401 that looks like a permissions problem.
const REFRESH_AT: f64 = 0.8;

/// Lifetime assumed when the exchange response omits one.
const FALLBACK_LIFETIME_SECS: u64 = 300;

/// Reads the current time, so cache expiry can be tested without sleeping.
pub trait Clock: Send + Sync {
    /// Seconds since an arbitrary fixed point. Only differences are used.
    fn now_secs(&self) -> u64;
}

/// The real clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// A local token held on behalf of one caller.
struct CachedToken {
    token: String,
    /// When to stop using it, already reduced by `REFRESH_AT`.
    refresh_after_secs: u64,
}

/// What the exchange endpoint returns.
#[derive(Deserialize)]
struct ExchangeData {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// The endpoint answers in the standard envelope.
#[derive(Deserialize)]
struct ExchangeEnvelope {
    data: Option<ExchangeData>,
}

/// Exchanges validated caller identities for short-lived local tokens.
pub struct OidcBackend {
    exchange_url: String,
    http: reqwest::Client,
    /// Keyed on a hash of the caller's token, never on anything shared.
    cache: RwLock<HashMap<String, CachedToken>>,
    clock: Arc<dyn Clock>,
}

impl OidcBackend {
    /// # Errors
    /// Returns `AuthError` if the HTTP client cannot be built.
    pub fn new(config: &Config) -> Result<Self, AuthError> {
        Self::with_clock(config, Arc::new(SystemClock))
    }

    /// # Errors
    /// As [`Self::new`].
    pub fn with_clock(config: &Config, clock: Arc<dyn Clock>) -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        let base = config.base_url.as_str().trim_end_matches('/');
        Ok(Self {
            exchange_url: format!("{base}/{EXCHANGE_PATH}"),
            http,
            cache: RwLock::new(HashMap::new()),
            clock: clock as Arc<dyn Clock>,
        })
    }

    /// A cached local token for this caller, if one is still fresh.
    async fn cached(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache
            .get(key)
            .filter(|entry| self.clock.now_secs() < entry.refresh_after_secs)
            .map(|entry| entry.token.clone())
    }

    /// Present the caller's token and take back a local one.
    async fn exchange(&self, caller_token: &str) -> Result<(String, u64), AuthError> {
        let response = self
            .http
            .post(&self.exchange_url)
            .json(&serde_json::json!({ "token": caller_token }))
            .send()
            .await
            .map_err(|e| AuthError::PermissionLookup(format!("token exchange failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            // The body may echo the presented token back. The status is enough
            // to act on and safe to keep.
            return Err(AuthError::InvalidToken(format!(
                "RDataCore refused the token exchange with {status}"
            )));
        }

        let envelope: ExchangeEnvelope = response.json().await.map_err(|e| {
            AuthError::PermissionLookup(format!("token exchange response was unreadable: {e}"))
        })?;

        let data = envelope.data.ok_or_else(|| {
            AuthError::PermissionLookup("token exchange returned no token".to_string())
        })?;

        Ok((
            data.access_token,
            data.expires_in.unwrap_or(FALLBACK_LIFETIME_SECS),
        ))
    }
}

#[async_trait::async_trait]
impl AuthBackend for OidcBackend {
    async fn outbound_headers(
        &self,
        ctx: &CallerContext,
    ) -> Result<Vec<(String, String)>, AuthError> {
        // No caller, no request. Deliberately not "no caller, use the
        // configured credential" — see the module comment.
        let bearer = ctx.bearer.as_deref().ok_or(AuthError::MissingCredential)?;

        if !ctx.validated {
            return Err(AuthError::InvalidToken(
                "the caller's token was not validated by the transport; refusing to exchange it"
                    .to_string(),
            ));
        }

        let key = caller_key(bearer);

        if let Some(token) = self.cached(&key).await {
            return Ok(bearer_header(&token));
        }

        let (token, lifetime) = self.exchange(bearer).await?;

        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let usable = (lifetime as f64 * REFRESH_AT) as u64;
        let entry = CachedToken {
            token: token.clone(),
            refresh_after_secs: self.clock.now_secs().saturating_add(usable),
        };
        self.cache.write().await.insert(key, entry);

        Ok(bearer_header(&token))
    }
}

fn bearer_header(token: &str) -> Vec<(String, String)> {
    vec![("Authorization".to_string(), format!("Bearer {token}"))]
}

/// Cache key for one caller.
///
/// A digest rather than the token itself, so a memory dump or a stray debug
/// print of the cache does not hand over a live credential. Two callers
/// colliding onto one entry would be impersonation rather than a cache miss,
/// which is why this is `SHA-256` and not the standard library's hasher.
fn caller_key(token: &str) -> String {
    use sha2::{Digest as _, Sha256};

    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod oidc_tests;
