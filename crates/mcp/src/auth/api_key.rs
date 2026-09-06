#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::{AuthBackend, AuthError, CallerContext};

/// The header `RDataCore` reads an API key from.
///
/// Verified against `extract_and_validate_api_key` in
/// `crates/api/src/auth/utils.rs`, which trims whitespace before validating.
const API_KEY_HEADER: &str = "X-API-Key";

/// Authenticates with a single static credential from the environment.
///
/// Development and single-user stdio only. Configuration refuses this backend
/// on the HTTP transport, because one shared credential over a network-facing
/// server would authenticate every caller as the same account and collapse the
/// per-user permissions the whole design depends on.
pub struct ApiKeyBackend {
    credential: String,
}

impl ApiKeyBackend {
    #[must_use]
    pub const fn new(credential: String) -> Self {
        Self { credential }
    }
}

#[async_trait::async_trait]
impl AuthBackend for ApiKeyBackend {
    async fn outbound_headers(
        &self,
        _ctx: &CallerContext,
    ) -> Result<Vec<(String, String)>, AuthError> {
        Ok(vec![(API_KEY_HEADER.to_string(), self.credential.clone())])
    }
}
