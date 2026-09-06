#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Fetching and caching an identity provider's signing keys.
//!
//! Lives here rather than in `core` because it makes network calls, and `core`
//! is a domain leaf with no HTTP dependency. It implements the `KeySource`
//! trait declared there, so validation stays pure and testable.
//!
//! The rate limit on forced refreshes is the part worth reading. Providers
//! rotate keys, so a token naming an unknown key id is normal and must trigger
//! a re-fetch. Without a limit, an attacker sending tokens with random key ids
//! turns this server into a load generator aimed at the identity provider — a
//! denial-of-service with someone else's infrastructure as the target.

use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use r_data_core_core::oidc::keys::{JwkSet, KeySource, KeySourceError};
use r_data_core_core::oidc::OidcConfig;

use crate::oidc_discovery;

/// Shortest gap between forced re-fetches, however many unknown key ids arrive.
const REFRESH_COOLDOWN: Duration = Duration::from_secs(60);

struct Cached {
    keys: JwkSet,
    fetched_at: Instant,
    /// When a forced refresh last happened, for the cooldown.
    last_forced: Option<Instant>,
}

/// Fetches key sets over HTTP and caches them.
pub struct HttpKeySource {
    client: reqwest::Client,
    issuer: String,
    ttl: Duration,
    cached: RwLock<Option<Cached>>,
}

impl HttpKeySource {
    /// # Errors
    /// Returns `KeySourceError` if the HTTP client cannot be built.
    pub fn new(config: &OidcConfig) -> Result<Self, KeySourceError> {
        Ok(Self {
            client: oidc_discovery::client()?,
            issuer: config.issuer.trim_end_matches('/').to_string(),
            ttl: config.jwks_ttl,
            cached: RwLock::new(None),
        })
    }

    /// Discover the key-set URL, then fetch it.
    async fn fetch(&self) -> Result<JwkSet, KeySourceError> {
        let discovery = oidc_discovery::fetch(&self.client, &self.issuer).await?;

        self.client
            .get(&discovery.jwks_uri)
            .send()
            .await
            .map_err(|e| KeySourceError::Unreachable(format!("{}: {e}", discovery.jwks_uri)))?
            .json::<JwkSet>()
            .await
            .map_err(|e| KeySourceError::Malformed(format!("{}: {e}", discovery.jwks_uri)))
    }

    async fn store(&self, keys: JwkSet, forced: bool) {
        let mut guard = self.cached.write().await;
        let last_forced = if forced {
            Some(Instant::now())
        } else {
            guard.as_ref().and_then(|c| c.last_forced)
        };
        *guard = Some(Cached {
            keys,
            fetched_at: Instant::now(),
            last_forced,
        });
    }
}

#[async_trait::async_trait]
impl KeySource for HttpKeySource {
    async fn keys(&self) -> Result<JwkSet, KeySourceError> {
        {
            let guard = self.cached.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.fetched_at.elapsed() < self.ttl {
                    return Ok(cached.keys.clone());
                }
            }
        }

        let keys = self.fetch().await?;
        self.store(keys.clone(), false).await;
        Ok(keys)
    }

    async fn refresh_for_unknown_kid(&self, kid: &str) -> Result<JwkSet, KeySourceError> {
        {
            let guard = self.cached.read().await;
            if let Some(cached) = guard.as_ref() {
                // Already have it — a concurrent refresh got there first.
                if cached.keys.find(kid).is_some() {
                    return Ok(cached.keys.clone());
                }
                // Rate limit. Returning the stale set means validation fails
                // with UnknownKid, which is the correct answer for a forged
                // key id and a brief, self-correcting one for a real rotation.
                if let Some(last) = cached.last_forced {
                    if last.elapsed() < REFRESH_COOLDOWN {
                        return Ok(cached.keys.clone());
                    }
                }
            }
        }

        let keys = self.fetch().await?;
        self.store(keys.clone(), true).await;
        Ok(keys)
    }
}

#[cfg(test)]
mod oidc_keys_tests;
