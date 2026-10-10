#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod app;
mod maintenance;
mod shared;
mod worker;

pub use app::load_app_config;
pub use maintenance::load_maintenance_config;
pub use worker::load_worker_config;

use crate::config::{CacheConfig, LicenseConfig};
use crate::error::Result;

/// Load `.env`, except under this crate's own unit tests.
///
/// The loaders are tested by setting process environment variables and
/// asserting on what comes back. `dotenvy` will not override a variable that
/// is already set, but it *will* set one a test deliberately removed — so a
/// test asserting "this is required and missing" passes only for a developer
/// whose `.env` happens not to contain that key. CI, which has no `.env`,
/// never sees the failure.
///
/// Integration tests in other crates still read `.env`, which is what they
/// want: they exercise a configured instance.
// Under `cfg(test)` the body is empty, so nursery suggests `const fn` — which
// the real build cannot be. The allow is scoped to the test build rather than
// applied unconditionally, so the suggestion still lands if this ever becomes
// const-able for real.
#[cfg_attr(test, allow(clippy::missing_const_for_fn))]
pub(super) fn load_dotenv() {
    #[cfg(not(test))]
    {
        dotenvy::dotenv().ok();
    }
}

/// Load license configuration from environment variables
///
/// This function loads the license configuration using the same logic as the main config loader.
/// It handles .env file loading and reads `LICENSE_KEY`, `LICENSE_PRIVATE_KEY`, `LICENSE_PUBLIC_KEY`, and uses default URLs.
///
/// # Errors
/// Returns an error if .env file loading fails (though this is usually non-fatal)
pub fn load_license_config() -> Result<LicenseConfig> {
    load_dotenv();

    Ok(shared::get_license_config())
}

/// Load cache configuration and Redis URL from environment variables
///
/// This function loads the cache configuration using the same logic as the maintenance config loader.
/// It handles .env file loading and reads cache settings and `REDIS_URL`.
///
/// # Errors
/// Returns an error if required environment variables are missing
pub fn load_cache_config() -> Result<(CacheConfig, String)> {
    load_dotenv();

    let cache = shared::get_cache_config();
    let redis_url = std::env::var("REDIS_URL")
        .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?;

    Ok((cache, redis_url))
}

#[cfg(test)]
#[path = "../loader_tests/mod.rs"]
mod tests;
