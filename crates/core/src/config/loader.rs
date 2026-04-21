#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use dotenvy::dotenv;
use std::env;

use crate::config::{
    ApiConfig, AppConfig, CacheConfig, DatabaseConfig, LicenseConfig, LogConfig, MailConfig,
    MaintenanceConfig, QueueConfig, WorkerConfig, WorkflowConfig,
};
use crate::error::Result;
use crate::utils;

/// Load application configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_app_config() -> Result<AppConfig> {
    // Load .env file if present
    dotenv().ok();

    let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let outbox_enabled = env::var("OUTBOX_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);
    let outbox_retry_base_delay_secs = env::var("OUTBOX_RETRY_BASE_DELAY_SECS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_BASE_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_base_delay_secs <= 0 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_BASE_DELAY_SECS must be > 0 seconds".to_string(),
        ));
    }
    let outbox_retry_multiplier = env::var("OUTBOX_RETRY_MULTIPLIER")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MULTIPLIER must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_multiplier < 2 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MULTIPLIER must be >= 2".to_string(),
        ));
    }
    let outbox_retry_max_delay_secs = env::var("OUTBOX_RETRY_MAX_DELAY_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MAX_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_max_delay_secs < outbox_retry_base_delay_secs {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MAX_DELAY_SECS must be >= OUTBOX_RETRY_BASE_DELAY_SECS".to_string(),
        ));
    }

    let database = DatabaseConfig {
        connection_string: env::var("DATABASE_URL")
            .map_err(|_| crate::error::Error::Config("DATABASE_URL not set".to_string()))?,
        max_connections: env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
        connection_timeout: env::var("DATABASE_CONNECTION_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30),
    };

    let api = ApiConfig {
        host: env::var("API_HOST")
            .or_else(|_| env::var("API_HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: env::var("API_PORT")
            .unwrap_or_else(|_| "8888".to_string())
            .parse()
            .unwrap_or(8888),
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
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        check_default_admin_password: env::var("CHECK_DEFAULT_ADMIN_PASSWORD")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
    };

    let log = LogConfig {
        level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        file: env::var("LOG_FILE").ok(),
    };

    let cache = get_cache_config();
    let queue = get_queue_config()?;
    let license = get_license_config();
    let mail = get_mail_config();

    Ok(AppConfig {
        environment,
        outbox_enabled,
        outbox_retry_base_delay_secs,
        outbox_retry_multiplier,
        outbox_retry_max_delay_secs,
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

/// Load worker configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_worker_config() -> Result<WorkerConfig> {
    // Ensure .env is loaded for binaries that only use WorkerConfig
    dotenv().ok();

    let interval_str = env::var("JOB_QUEUE_UPDATE_INTERVAL").map_err(|_| {
        crate::error::Error::Config("JOB_QUEUE_UPDATE_INTERVAL not set".to_string())
    })?;
    let job_queue_update_interval_secs = interval_str.parse::<u64>().map_err(|_| {
        crate::error::Error::Config(
            "JOB_QUEUE_UPDATE_INTERVAL must be a positive integer (seconds)".to_string(),
        )
    })?;
    if job_queue_update_interval_secs == 0 {
        return Err(crate::error::Error::Config(
            "JOB_QUEUE_UPDATE_INTERVAL must be > 0 seconds".to_string(),
        ));
    }
    let outbox_enabled = env::var("OUTBOX_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);
    let outbox_retry_base_delay_secs = env::var("OUTBOX_RETRY_BASE_DELAY_SECS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_BASE_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_base_delay_secs <= 0 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_BASE_DELAY_SECS must be > 0 seconds".to_string(),
        ));
    }
    let outbox_retry_multiplier = env::var("OUTBOX_RETRY_MULTIPLIER")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MULTIPLIER must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_multiplier < 2 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MULTIPLIER must be >= 2".to_string(),
        ));
    }
    let outbox_retry_max_delay_secs = env::var("OUTBOX_RETRY_MAX_DELAY_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MAX_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_retry_max_delay_secs < outbox_retry_base_delay_secs {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MAX_DELAY_SECS must be >= OUTBOX_RETRY_BASE_DELAY_SECS".to_string(),
        ));
    }
    let outbox_stale_lease_secs = env::var("OUTBOX_STALE_LEASE_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_STALE_LEASE_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if outbox_stale_lease_secs <= 0 {
        return Err(crate::error::Error::Config(
            "OUTBOX_STALE_LEASE_SECS must be > 0 seconds".to_string(),
        ));
    }

    let database = DatabaseConfig {
        connection_string: env::var("WORKER_DATABASE_URL")
            .map_err(|_| crate::error::Error::Config("WORKER_DATABASE_URL not set".to_string()))?,
        max_connections: env::var("WORKER_DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
        connection_timeout: env::var("DATABASE_CONNECTION_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30),
    };

    let workflow = WorkflowConfig {
        worker_threads: env::var("WORKFLOW_WORKER_THREADS")
            .unwrap_or_else(|_| "4".to_string())
            .parse()
            .unwrap_or(4),
        default_timeout: env::var("WORKFLOW_DEFAULT_TIMEOUT")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300),
        max_concurrent: env::var("WORKFLOW_MAX_CONCURRENT")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
    };

    let queue = get_queue_config()?;
    let cache = get_cache_config();
    let license = get_license_config();
    let mail = get_mail_config();

    Ok(WorkerConfig {
        job_queue_update_interval_secs,
        outbox_enabled,
        outbox_stale_lease_secs,
        outbox_retry_base_delay_secs,
        outbox_retry_multiplier,
        outbox_retry_max_delay_secs,
        database,
        workflow,
        queue,
        cache,
        license,
        mail,
    })
}

/// Load maintenance configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_maintenance_config() -> Result<MaintenanceConfig> {
    // Ensure .env is loaded for binaries that only use MaintenanceConfig
    dotenv().ok();
    let outbox_enabled = env::var("OUTBOX_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);
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

fn load_required_cron(name: &str) -> Result<String> {
    let cron =
        env::var(name).map_err(|_| crate::error::Error::Config(format!("{name} not set")))?;
    utils::validate_cron(&cron)
        .map_err(|e| crate::error::Error::Config(format!("Invalid {name} '{cron}': {e}")))?;
    Ok(cron)
}

fn load_retention_days<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr + ToString + Copy,
{
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|_| crate::error::Error::Config(format!("{name} must be a valid number")))
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

fn get_cache_config() -> CacheConfig {
    CacheConfig {
        enabled: env::var("CACHE_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
        ttl: env::var("CACHE_TTL")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300),
        max_size: env::var("CACHE_MAX_SIZE")
            .unwrap_or_else(|_| "10000".to_string())
            .parse()
            .unwrap_or(10000),
        entity_definition_ttl: env::var("CACHE_ENTITY_DEFINITION_TTL")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0),
        api_key_ttl: env::var("CACHE_API_KEY_TTL")
            .unwrap_or_else(|_| "600".to_string())
            .parse()
            .unwrap_or(600),
    }
}

fn get_queue_config() -> Result<QueueConfig> {
    let config = QueueConfig {
        redis_url: env::var("REDIS_URL")
            .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?,
        fetch_key: env::var("QUEUE_FETCH_KEY")
            .unwrap_or_else(|_| "queue:workflows:fetch".to_string()),
        process_key: env::var("QUEUE_PROCESS_KEY")
            .unwrap_or_else(|_| "queue:workflows:process".to_string()),
        email_key: env::var("QUEUE_EMAIL_KEY").unwrap_or_else(|_| "queue:email".to_string()),
    };

    Ok(config)
}

fn get_mail_config() -> MailConfig {
    let system = std::env::var("SYSTEM_SMTP_DSN")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|dsn| {
            crate::config::mail::parse_smtp_dsn(&dsn)
                .map_err(|e| {
                    log::warn!("Failed to parse SYSTEM_SMTP_DSN: {e}");
                    e
                })
                .ok()
        });

    let workflow = std::env::var("WORKFLOW_SMTP_DSN")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|dsn| {
            crate::config::mail::parse_smtp_dsn(&dsn)
                .map_err(|e| {
                    log::warn!("Failed to parse WORKFLOW_SMTP_DSN: {e}");
                    e
                })
                .ok()
        });

    MailConfig { system, workflow }
}

fn get_license_config() -> LicenseConfig {
    LicenseConfig {
        license_key: env::var("LICENSE_KEY").ok(),
        private_key: env::var("LICENSE_PRIVATE_KEY").ok(),
        public_key: env::var("LICENSE_PUBLIC_KEY").ok(),
        verification_url: LicenseConfig::default().verification_url,
        statistics_url: LicenseConfig::default().statistics_url,
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
    // Load .env file if present (same as other config loaders)
    dotenv().ok();

    Ok(get_license_config())
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
    dotenv().ok();

    let cache = get_cache_config();
    let redis_url = env::var("REDIS_URL")
        .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?;

    Ok((cache, redis_url))
}
