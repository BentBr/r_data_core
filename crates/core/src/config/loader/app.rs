#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use dotenvy::dotenv;
use std::env;

use crate::config::AppConfig;
use crate::error::Result;

use super::shared::{
    get_cache_config, get_license_config, get_mail_config, get_queue_config, load_database_config,
    load_log_config, load_outbox_config, load_runtime_api_config,
};

/// Load application configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_app_config() -> Result<AppConfig> {
    // Load .env file if present
    dotenv().ok();

    let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let outbox = load_outbox_config(false)?;
    let database = load_database_config()?;
    let api = load_runtime_api_config()?;
    let log = load_log_config();

    let cache = get_cache_config();
    let queue = get_queue_config()?;
    let license = get_license_config();
    let mail = get_mail_config();

    Ok(AppConfig {
        environment,
        outbox_enabled: outbox.enabled,
        outbox_fetch_enabled: outbox.fetch_enabled,
        outbox_push_enabled: outbox.push_enabled,
        outbox_retry_base_delay_secs: outbox.retry_base_delay_secs,
        outbox_retry_multiplier: outbox.retry_multiplier,
        outbox_retry_max_delay_secs: outbox.retry_max_delay_secs,
        database,
        api,
        cache,
        log,
        queue,
        license,
        mail,
        frontend_base_url: env::var("FRONTEND_BASE_URL").ok().filter(|s| !s.is_empty()),
        password_reset_throttle_seconds: env::var("PASSWORD_RESET_THROTTLE_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60),
    })
}
