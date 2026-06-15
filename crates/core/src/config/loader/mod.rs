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

/// Load license configuration from environment variables
///
/// This function loads the license configuration using the same logic as the main config loader.
/// It handles .env file loading and reads `LICENSE_KEY`, `LICENSE_PRIVATE_KEY`, `LICENSE_PUBLIC_KEY`, and uses default URLs.
///
/// # Errors
/// Returns an error if .env file loading fails (though this is usually non-fatal)
pub fn load_license_config() -> Result<LicenseConfig> {
    // Load .env file if present (same as other config loaders)
    dotenvy::dotenv().ok();

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
    // Load .env file if present (same as other config loaders)
    dotenvy::dotenv().ok();

    let cache = shared::get_cache_config();
    let redis_url = std::env::var("REDIS_URL")
        .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?;

    Ok((cache, redis_url))
}

#[cfg(test)]
#[path = "../loader_tests/mod.rs"]
mod tests;
