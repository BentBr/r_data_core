#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Asking a provider where its endpoints are.
//!
//! One place, because two parts of this system need the answer: the key
//! source wants `jwks_uri`, and the login flow wants the authorization and
//! token endpoints. Two discovery implementations would drift, and the
//! failure that produces — half the system following a provider that has
//! moved, half of it not — is a miserable one to diagnose.
//!
//! Discovery is repeated per use rather than cached. It is one extra request
//! against a provider that is already being contacted, and it means a
//! provider that relocates an endpoint is followed rather than leaving this
//! server pointed at a dead URL.

use std::time::Duration;

use serde::Deserialize;

use r_data_core_core::oidc::keys::KeySourceError;

/// Timeout for a call to the identity provider.
///
/// Short on purpose: this sits in the authentication path, and a hung
/// provider should fail requests quickly rather than exhaust the pool.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// The subset of OIDC discovery this system reads.
///
/// Everything else the document carries is ignored. A field that is not read
/// cannot influence where this server sends a credential.
#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
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

/// Fetch a provider's discovery document.
///
/// # Errors
/// Returns `KeySourceError` when the provider cannot be reached or its
/// response is not a usable discovery document.
pub async fn fetch(http: &reqwest::Client, issuer: &str) -> Result<Discovery, KeySourceError> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    http.get(&url)
        .send()
        .await
        .map_err(|e| KeySourceError::Unreachable(format!("{url}: {e}")))?
        .json::<Discovery>()
        .await
        .map_err(|e| KeySourceError::Malformed(format!("{url}: {e}")))
}
