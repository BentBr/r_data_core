#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use dotenvy::dotenv;
use std::env;

use crate::config::{ApiConfig, DatabaseConfig, MaintenanceConfig};
use crate::error::Result;

use super::shared::{
    get_cache_config, get_license_config, load_required_cron, load_retention_days,
};

/// Load maintenance configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_maintenance_config() -> Result<MaintenanceConfig> {
    // Ensure .env is loaded for binaries that only use MaintenanceConfig
    dotenv().ok();
    let outbox_enabled = env::var("OUTBOX_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);
    let version_purger_cron = load_required_cron("VERSION_PURGER_CRON")?;
    let refresh_token_cleanup_cron = load_required_cron("REFRESH_TOKEN_CLEANUP_CRON")?;
    let workflow_run_logs_purger_cron = load_required_cron("WORKFLOW_RUN_LOGS_PURGER_CRON")?;
    let (outbox_purger_cron, outbox_retention_days) =
        load_outbox_maintenance_config(outbox_enabled)?;
    let system_logs_purger_cron = load_required_cron("SYSTEM_LOGS_PURGER_CRON")?;
    let system_logs_retention_days = load_retention_days("SYSTEM_LOGS_RETENTION_DAYS", 90_u64)?;
    let database = load_maintenance_database_config()?;

    let cache = get_cache_config();
    let redis_url = env::var("REDIS_URL")
        .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?;
    let license = get_license_config();
    let api = load_maintenance_api_config()?;

    Ok(MaintenanceConfig {
        outbox_enabled,
        version_purger_cron,
        refresh_token_cleanup_cron,
        workflow_run_logs_purger_cron,
        system_logs_purger_cron,
        system_logs_retention_days,
        outbox_purger_cron,
        outbox_retention_days,
        database,
        cache,
        redis_url,
        license,
        api,
    })
}

fn load_outbox_maintenance_config(outbox_enabled: bool) -> Result<(Option<String>, Option<u32>)> {
    if !outbox_enabled {
        return Ok((None, None));
    }

    let outbox_purger_cron = load_required_cron("OUTBOX_PURGER_CRON")?;
    let outbox_retention_days = load_retention_days("OUTBOX_RETENTION_DAYS", 30_u32)?;

    Ok((Some(outbox_purger_cron), Some(outbox_retention_days)))
}

fn load_maintenance_database_config() -> Result<DatabaseConfig> {
    Ok(DatabaseConfig {
        connection_string: env::var("MAINTENANCE_DATABASE_URL").map_err(|_| {
            crate::error::Error::Config("MAINTENANCE_DATABASE_URL not set".to_string())
        })?,
        max_connections: env::var("MAINTENANCE_DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
        connection_timeout: env::var("MAINTENANCE_DATABASE_CONNECTION_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30),
    })
}

fn load_maintenance_api_config() -> Result<ApiConfig> {
    Ok(ApiConfig {
        host: env::var("API_HOST")
            .map_err(|_| crate::error::Error::Config("API_HOST not set".to_string()))?,
        port: env::var("API_PORT")
            .map_err(|_| crate::error::Error::Config("API_PORT not set".to_string()))?
            .parse()
            .map_err(|_| {
                crate::error::Error::Config("API_PORT must be a valid number".to_string())
            })?,
        use_tls: env::var("API_USE_TLS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false),
        jwt_secret: env::var("JWT_SECRET")
            .map_err(|_| crate::error::Error::Config("JWT_SECRET not set".to_string()))?,
        jwt_expiration: env::var("JWT_EXPIRATION")
            .unwrap_or_else(|_| "86400".to_string())
            .parse()
            .unwrap_or(86400),
        enable_docs: env::var("API_ENABLE_DOCS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
        cors_origins: env::var("CORS_ORIGINS")
            .map_err(|_| crate::error::Error::Config("CORS_ORIGINS not set".to_string()))?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        check_default_admin_password: env::var("CHECK_DEFAULT_ADMIN_PASSWORD")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
    })
}
