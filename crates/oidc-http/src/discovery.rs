#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Asking a provider where its endpoints are.
//!
//! One place, because two parts of this system need the answer: the key
//! source wants `jwks_uri`, and the login flow wants the authorization and
//! token endpoints. Two discovery implementations would drift, and the
//! failure that produces — half the system following a provider that has
//! moved, half of it not — is a miserable one to diagnose.
//!
//! The document is cached for [`DISCOVERY_TTL`]. Both legs of a sign-in must
//! see the same endpoints, and the TTL is short enough that a provider that
//! relocates an endpoint is followed within minutes rather than leaving this
//! server pointed at a dead URL.

use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::RwLock;

use r_data_core_core::oidc::keys::KeySourceError;

/// Timeout for a call to the identity provider.
///
/// Short on purpose: this sits in the authentication path, and a hung
/// provider should fail requests quickly rather than exhaust the pool.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Largest discovery document this server will read.
///
/// Real documents are a few kilobytes. A provider — or whoever answers for
/// it — that sends more is not describing endpoints.
pub const DISCOVERY_MAX_BYTES: usize = 1 << 20;
/// Largest key set this server will read. Same reasoning.
pub const JWKS_MAX_BYTES: usize = 1 << 20;
/// How long a fetched document is reused.
///
/// Long enough that `start` and the code exchange five minutes later see
/// the same endpoints; short enough that a provider that moves one is
/// followed within minutes.
pub const DISCOVERY_TTL: Duration = Duration::from_secs(300);

/// The subset of OIDC discovery this system reads.
///
/// Everything else the document carries is ignored. A field that is not read
/// cannot influence where this server sends a credential.
#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
    /// Must equal the issuer the document was fetched for. A document that
    /// names another issuer was served by something other than the provider
    /// this instance trusts.
    pub issuer: String,
    pub jwks_uri: String,
    /// Absent from some providers' documents, and only the login flow needs it.
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
}

/// Build a client with the authentication-path timeout.
///
/// # Errors
/// Returns `KeySourceError::Unreachable` if the client cannot be built.
pub fn client() -> Result<reqwest::Client, KeySourceError> {
    reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| KeySourceError::Unreachable(e.to_string()))
}

/// Fetch a provider's discovery document and check it before trusting it.
///
/// `allow_insecure` permits plain-HTTP endpoints; loopback endpoints are
/// always permitted, as for the issuer itself.
///
/// # Errors
/// Returns `KeySourceError::Unreachable` when the provider cannot be reached
/// and `Malformed` when the document is too large, not JSON, names a
/// different issuer, or points at endpoints this server will not send a
/// credential to.
pub async fn fetch(
    http: &reqwest::Client,
    issuer: &str,
    allow_insecure: bool,
) -> Result<Discovery, KeySourceError> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let body = read_capped(http.get(&url), DISCOVERY_MAX_BYTES, &url).await?;
    let discovery: Discovery = serde_json::from_slice(&body)
        .map_err(|e| KeySourceError::Malformed(format!("{url}: {e}")))?;

    if discovery.issuer.trim_end_matches('/') != issuer.trim_end_matches('/') {
        return Err(KeySourceError::Malformed(format!(
            "{url}: the document names a different issuer than it was fetched for"
        )));
    }
    let endpoints = [
        Some(&discovery.jwks_uri),
        discovery.authorization_endpoint.as_ref(),
        discovery.token_endpoint.as_ref(),
    ];
    for endpoint in endpoints.into_iter().flatten() {
        if !endpoint.starts_with("https://")
            && !allow_insecure
            && !r_data_core_core::oidc::is_loopback_issuer(endpoint)
        {
            return Err(KeySourceError::Malformed(format!(
                "{url}: the document points at a plain-HTTP endpoint, which this server will \
                 not send a credential to"
            )));
        }
    }
    Ok(discovery)
}

/// Send a request and read at most `max` bytes of its body.
///
/// # Errors
/// `Unreachable` on transport failure or a non-success status; `Malformed`
/// when the body exceeds `max`.
pub async fn read_capped(
    request: reqwest::RequestBuilder,
    max: usize,
    url: &str,
) -> Result<Vec<u8>, KeySourceError> {
    let response = request
        .send()
        .await
        .map_err(|e| KeySourceError::Unreachable(format!("{url}: {e}")))?;
    if !response.status().is_success() {
        return Err(KeySourceError::Unreachable(format!(
            "{url}: answered {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|len| len > max as u64)
    {
        return Err(KeySourceError::Malformed(format!(
            "{url}: body exceeds {max} bytes"
        )));
    }
    // Stream, so a hostile response with no length is never held whole.
    let mut response = response;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| KeySourceError::Unreachable(format!("{url}: {e}")))?
    {
        if body.len() + chunk.len() > max {
            return Err(KeySourceError::Malformed(format!(
                "{url}: body exceeds {max} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// A discovery document, reused for [`DISCOVERY_TTL`].
pub struct DiscoveryCache {
    cached: RwLock<Option<(Discovery, Instant)>>,
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cached: RwLock::new(None),
        }
    }

    /// The document, fetched if the cached one is absent or stale.
    ///
    /// # Errors
    /// As [`fetch`].
    pub async fn get(
        &self,
        http: &reqwest::Client,
        issuer: &str,
        allow_insecure: bool,
    ) -> Result<Discovery, KeySourceError> {
        {
            let guard = self.cached.read().await;
            if let Some((doc, at)) = guard.as_ref() {
                if at.elapsed() < DISCOVERY_TTL {
                    return Ok(doc.clone());
                }
            }
        }
        let fresh = fetch(http, issuer, allow_insecure).await?;
        *self.cached.write().await = Some((fresh.clone(), Instant::now()));
        Ok(fresh)
    }
}

#[cfg(test)]
mod failure_tests;
#[cfg(test)]
mod tests;
