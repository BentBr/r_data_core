#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The one path from an identity-provider token to an `RDataCore` session.
//!
//! Three things authenticate through OIDC: the bearer-token middleware, the
//! browser login callback, and the machine token exchange. All three end up
//! here, and that is deliberate — the account-status check, the role mapping
//! and the claim minting are the parts that must not differ between them. A
//! second implementation of this tail is how a deactivated user keeps working
//! on one route and not another.
//!
//! Two decisions are worth reading before changing anything.
//!
//! **The token is verified on every request; only the database lookup is
//! cached.** Caching the whole authentication keyed on the token would mean an
//! expired token kept working until the cache entry aged out. Verification is
//! a signature check against keys already in memory, so it is cheap; resolving
//! the user is the round trip worth avoiding.
//!
//! **The cache window is how long a revocation takes to bite.** A user
//! deactivated in `RDataCore` keeps their access for at most
//! `resolution_cache_ttl` (60 seconds by default). That is the trade being
//! made, and it is why the default is short rather than convenient.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

use r_data_core_core::admin_jwt::{build_claims, AuthUserClaims};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Error as CoreError;
use r_data_core_core::oidc::keys::{KeySource, KeySourceError, OidcClaims};
use r_data_core_core::oidc::{validate_token, OidcConfig, ValidationError};

use crate::oidc_provisioning::OidcProvisioningService;

/// Cache-key prefix for resolved identities.
const RESOLUTION_PREFIX: &str = "oidc:resolve:";

#[derive(Debug, Error)]
pub enum OidcAuthError {
    /// The token itself is not acceptable.
    #[error("the token was not accepted: {0}")]
    Rejected(#[from] ValidationError),
    /// The provider's keys could not be obtained.
    #[error("the identity provider's signing keys are unavailable: {0}")]
    Keys(#[from] KeySourceError),
    /// The token is valid but this person may not act.
    #[error("{0}")]
    Denied(String),
    /// Something this server depends on is broken. Distinct from `Denied`
    /// because a database outage is a 503, not a 403 — reporting it as a
    /// refusal would send an operator looking at role mappings.
    #[error("authentication is temporarily unavailable: {0}")]
    Unavailable(String),
}

impl From<CoreError> for OidcAuthError {
    fn from(error: CoreError) -> Self {
        match error {
            // The provisioning service reports every refusal as `Auth`.
            CoreError::Auth(message) => Self::Denied(message),
            other => Self::Unavailable(other.to_string()),
        }
    }
}

/// What gets cached between requests: the outcome of resolving an identity.
#[derive(Serialize, Deserialize)]
struct CachedResolution {
    claims: AuthUserClaims,
}

/// Everything the OIDC paths need, wired together once at startup.
pub struct OidcRuntime {
    config: OidcConfig,
    keys: Arc<dyn KeySource>,
    provisioning: Arc<OidcProvisioningService>,
    cache: Arc<CacheManager>,
}

impl OidcRuntime {
    #[must_use]
    pub const fn new(
        config: OidcConfig,
        keys: Arc<dyn KeySource>,
        provisioning: Arc<OidcProvisioningService>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            config,
            keys,
            provisioning,
            cache,
        }
    }

    #[must_use]
    pub const fn config(&self) -> &OidcConfig {
        &self.config
    }

    /// Whether this token claims to come from the issuer this instance trusts.
    ///
    /// **This reads the payload without verifying the signature**, and is
    /// sound only because of what it is used for: choosing which validator
    /// runs. A forged `iss` lets an attacker pick which of two validators
    /// rejects their token, and nothing else. The value never reaches role
    /// mapping, provisioning, or a log line that presents it as fact.
    #[must_use]
    pub fn token_targets_this_issuer(&self, token: &str) -> bool {
        unverified_issuer(token).is_some_and(|unverified_iss| unverified_iss == self.config.issuer)
    }

    /// Verify a token's signature and claims.
    ///
    /// Refreshes the key set once when the token names a key id the cached set
    /// does not have — providers rotate keys, and a single retry is the
    /// difference between following a rotation and rejecting everyone until
    /// the cache expires.
    ///
    /// # Errors
    /// Returns `OidcAuthError` when the token is not acceptable or the
    /// provider's keys cannot be fetched.
    pub async fn verify(&self, token: &str, now: i64) -> Result<OidcClaims, OidcAuthError> {
        let keys = self.keys.keys().await?;
        match validate_token(token, &keys, &self.config, now) {
            Err(ValidationError::UnknownKid { kid }) => {
                let refreshed = self.keys.refresh_for_unknown_kid(&kid).await?;
                Ok(validate_token(token, &refreshed, &self.config, now)?)
            }
            other => Ok(other?),
        }
    }

    /// Resolve verified claims into the claims an `RDataCore` session carries.
    ///
    /// This is the shared tail. The bearer path reaches it through
    /// [`Self::authenticate`]; the login callback and the token exchange call
    /// it directly, having verified an ID token by their own route.
    ///
    /// # Errors
    /// Returns `OidcAuthError::Denied` when the identity maps to no role or
    /// the account may not sign in, and `Unavailable` when the database is
    /// not reachable.
    pub async fn session_claims(
        &self,
        claims: &OidcClaims,
        session_seconds: u64,
    ) -> Result<AuthUserClaims, OidcAuthError> {
        let key = resolution_key(&claims.issuer, &claims.subject);

        if let Ok(Some(hit)) = self.cache.get::<CachedResolution>(&key).await {
            return Ok(hit.claims);
        }

        let (user, roles) = self
            .provisioning
            .resolve_or_provision(claims, &self.config)
            .await?;

        let minted = build_claims(&user, &roles, session_seconds)?;

        // A cache failure must not fail the request: the entry is an
        // optimisation, and the authoritative answer is already in hand.
        if let Err(e) = self
            .cache
            .set(
                &key,
                &CachedResolution {
                    claims: minted.clone(),
                },
                Some(self.config.resolution_cache_ttl.as_secs()),
            )
            .await
        {
            log::warn!("could not cache the resolved OIDC identity: {e}");
        }

        Ok(minted)
    }

    /// Verify a bearer token and resolve it to session claims.
    ///
    /// # Errors
    /// As [`Self::verify`] and [`Self::session_claims`].
    pub async fn authenticate(
        &self,
        token: &str,
        now: i64,
        session_seconds: u64,
    ) -> Result<AuthUserClaims, OidcAuthError> {
        let verified = self.verify(token, now).await?;
        self.session_claims(&verified, session_seconds).await
    }

    /// Forget the cached resolution for one identity.
    pub async fn forget(&self, issuer: &str, subject: &str) {
        if let Err(e) = self.cache.delete(&resolution_key(issuer, subject)).await {
            log::warn!("could not evict the cached OIDC identity: {e}");
        }
    }

    /// Forget every cached resolution belonging to one account.
    ///
    /// Called when an account is deactivated, locked, or has its roles
    /// changed. Without it those changes take effect only after the cache
    /// window — which is the documented behaviour, but a deactivation is
    /// precisely the case where waiting a minute is unwelcome.
    ///
    /// The cache is keyed on the identity, not the account, so the identities
    /// have to be looked up to know what to evict.
    pub async fn forget_user(&self, admin_user_uuid: uuid::Uuid) {
        match self.provisioning.identities_for(admin_user_uuid).await {
            Ok(identities) => {
                for (provider, subject) in identities {
                    self.forget(&provider, &subject).await;
                }
            }
            Err(e) => log::warn!("could not read identities to evict for an account: {e}"),
        }
    }
}

/// Cache key for one identity.
///
/// Hashed so that a subject containing anything at all — a colon, a newline,
/// something enormous — cannot shape the key space.
fn resolution_key(issuer: &str, subject: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(issuer.as_bytes());
    hasher.update([0]);
    hasher.update(subject.as_bytes());
    format!("{RESOLUTION_PREFIX}{:x}", hasher.finalize())
}

/// Read `iss` from a token's payload **without verifying anything**.
///
/// Named for what it is. See [`OidcRuntime::token_targets_this_issuer`] for
/// why reading unverified claims is acceptable in this one place.
fn unverified_issuer(token: &str) -> Option<String> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;

    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let parsed: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    parsed
        .get("iss")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod oidc_runtime_tests;
