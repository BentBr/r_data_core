#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Authenticating with a single static API key.
//!
//! Development and single-user stdio only. Configuration refuses this backend
//! on the HTTP transport, because one shared credential over a network-facing
//! server would authenticate every caller as the same account and collapse
//! the per-user permissions the whole design depends on.
//!
//! **The key is exchanged, not sent.** `RDataCore`'s admin API requires a JWT
//! — an API key authenticates the public API only — so this presents the key
//! at `/auth/api-key/token` and uses the short-lived admin token it gets back.
//! An earlier version sent `X-API-Key` to admin endpoints directly and was
//! rejected by every one of them; the tests that would have caught it existed
//! but had never run.
//!
//! Structurally this is the same as `OidcBackend`: exchange a credential for a
//! local token, cache it, refresh before it expires.

use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

use super::{AuthBackend, AuthError, CallerContext};
use crate::auth::oidc::{Clock, SystemClock};
use crate::config::Config;

/// The header `RDataCore` reads an API key from.
///
/// Verified against `extract_and_validate_api_key` in
/// `crates/api/src/auth/utils.rs`, which trims whitespace before validating.
const API_KEY_HEADER: &str = "X-API-Key";

/// Path of the exchange endpoint.
const EXCHANGE_PATH: &str = "admin/api/v1/auth/api-key/token";

/// Fraction of a token's life after which it is exchanged again.
const REFRESH_AT: f64 = 0.8;

/// Lifetime assumed when the response omits one.
const FALLBACK_LIFETIME_SECS: u64 = 300;

#[derive(Deserialize)]
struct ExchangeData {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct ExchangeEnvelope {
    data: Option<ExchangeData>,
}

/// The admin token currently held, and when to replace it.
struct HeldToken {
    token: String,
    refresh_after_secs: u64,
}

/// Authenticates with a single static API key, exchanged for admin tokens.
pub struct ApiKeyBackend {
    credential: String,
    exchange_url: String,
    http: reqwest::Client,
    held: RwLock<Option<HeldToken>>,
    clock: Arc<dyn Clock>,
}

impl ApiKeyBackend {
    /// # Errors
    /// Returns `AuthError` if the HTTP client cannot be built.
    pub fn new(config: &Config, credential: String) -> Result<Self, AuthError> {
        Self::with_clock(config, credential, Arc::new(SystemClock))
    }

    /// # Errors
    /// As [`Self::new`].
    pub fn with_clock(
        config: &Config,
        credential: String,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        let base = config.base_url.as_str().trim_end_matches('/');
        Ok(Self {
            credential,
            exchange_url: format!("{base}/{EXCHANGE_PATH}"),
            http,
            held: RwLock::new(None),
            clock,
        })
    }

    /// A backend that already holds a token, for tests about something else.
    ///
    /// The exchange has its own tests. Every test that merely needs an
    /// authenticated client would otherwise have to mock the exchange too,
    /// and one that forgot would fail with "credential invalid" rather than
    /// anything pointing at the omission.
    #[cfg(test)]
    pub(crate) fn holding(config: &Config, token: &str) -> Result<Self, AuthError> {
        let backend = Self::new(config, "unused-in-this-mode".to_string())?;
        *backend
            .held
            .try_write()
            .map_err(|_| AuthError::InvalidToken("lock".to_string()))? = Some(HeldToken {
            token: token.to_string(),
            refresh_after_secs: u64::MAX,
        });
        Ok(backend)
    }

    /// The held token, if it is still fresh.
    async fn cached(&self) -> Option<String> {
        let held = self.held.read().await;
        held.as_ref()
            .filter(|held| self.clock.now_secs() < held.refresh_after_secs)
            .map(|held| held.token.clone())
    }

    /// Present the key and take back an admin token.
    async fn exchange(&self) -> Result<(String, u64), AuthError> {
        let response = self
            .http
            .post(&self.exchange_url)
            .header(API_KEY_HEADER, &self.credential)
            .send()
            .await
            .map_err(|e| {
                AuthError::PermissionLookup(format!(
                    "could not reach RDataCore to exchange the API key: {e}"
                ))
            })?;

        let status = response.status();
        if !status.is_success() {
            // The status is enough to act on, and the body may echo the key.
            return Err(AuthError::InvalidToken(format!(
                "RDataCore refused the API key with {status}. Check RDC_API_KEY and that \
                 the key has not been revoked."
            )));
        }

        let envelope: ExchangeEnvelope = response.json().await.map_err(|e| {
            AuthError::PermissionLookup(format!("the token exchange response was unreadable: {e}"))
        })?;
        let data = envelope.data.ok_or_else(|| {
            AuthError::PermissionLookup("the token exchange returned no token".to_string())
        })?;

        Ok((
            data.access_token,
            data.expires_in.unwrap_or(FALLBACK_LIFETIME_SECS),
        ))
    }
}

#[async_trait::async_trait]
impl AuthBackend for ApiKeyBackend {
    async fn outbound_headers(
        &self,
        _ctx: &CallerContext,
    ) -> Result<Vec<(String, String)>, AuthError> {
        if let Some(token) = self.cached().await {
            return Ok(bearer(&token));
        }

        let (token, lifetime) = self.exchange().await?;

        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let usable = (lifetime as f64 * REFRESH_AT) as u64;
        *self.held.write().await = Some(HeldToken {
            token: token.clone(),
            refresh_after_secs: self.clock.now_secs().saturating_add(usable),
        });

        Ok(bearer(&token))
    }
}

fn bearer(token: &str) -> Vec<(String, String)> {
    vec![("Authorization".to_string(), format!("Bearer {token}"))]
}

#[cfg(test)]
mod api_key_tests;
