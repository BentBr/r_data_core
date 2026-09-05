#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::env;

use crate::config::{
    ApiConfig, CacheConfig, DatabaseConfig, LicenseConfig, LogConfig, MailConfig, QueueConfig,
    WorkflowConfig,
};
use crate::error::Result;
use crate::utils;

pub(super) struct OutboxConfigVars {
    pub(super) enabled: bool,
    pub(super) fetch_enabled: bool,
    pub(super) push_enabled: bool,
    pub(super) stale_lease_secs: i64,
    pub(super) retry_base_delay_secs: i64,
    pub(super) retry_multiplier: u64,
    pub(super) retry_max_delay_secs: i64,
}

pub(super) fn load_outbox_config(include_stale_lease: bool) -> Result<OutboxConfigVars> {
    let enabled = env::var("OUTBOX_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);
    let fetch_enabled = env::var("OUTBOX_FETCH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);
    let push_enabled = env::var("OUTBOX_PUSH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let retry_base_delay_secs = env::var("OUTBOX_RETRY_BASE_DELAY_SECS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_BASE_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if retry_base_delay_secs <= 0 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_BASE_DELAY_SECS must be > 0 seconds".to_string(),
        ));
    }

    let retry_multiplier = env::var("OUTBOX_RETRY_MULTIPLIER")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MULTIPLIER must be a valid positive integer".to_string(),
            )
        })?;
    if retry_multiplier < 2 {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MULTIPLIER must be >= 2".to_string(),
        ));
    }

    let retry_max_delay_secs = env::var("OUTBOX_RETRY_MAX_DELAY_SECS")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<i64>()
        .map_err(|_| {
            crate::error::Error::Config(
                "OUTBOX_RETRY_MAX_DELAY_SECS must be a valid positive integer".to_string(),
            )
        })?;
    if retry_max_delay_secs < retry_base_delay_secs {
        return Err(crate::error::Error::Config(
            "OUTBOX_RETRY_MAX_DELAY_SECS must be >= OUTBOX_RETRY_BASE_DELAY_SECS".to_string(),
        ));
    }

    let stale_lease_secs = if include_stale_lease {
        let secs = env::var("OUTBOX_STALE_LEASE_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<i64>()
            .map_err(|_| {
                crate::error::Error::Config(
                    "OUTBOX_STALE_LEASE_SECS must be a valid positive integer".to_string(),
                )
            })?;
        if secs <= 0 {
            return Err(crate::error::Error::Config(
                "OUTBOX_STALE_LEASE_SECS must be > 0 seconds".to_string(),
            ));
        }
        secs
    } else {
        300
    };

    Ok(OutboxConfigVars {
        enabled,
        fetch_enabled,
        push_enabled,
        stale_lease_secs,
        retry_base_delay_secs,
        retry_multiplier,
        retry_max_delay_secs,
    })
}

pub(super) fn load_database_config() -> Result<DatabaseConfig> {
    Ok(DatabaseConfig {
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
    })
}

pub(super) fn load_worker_database_config() -> Result<DatabaseConfig> {
    Ok(DatabaseConfig {
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
    })
}

pub(super) fn load_runtime_api_config() -> Result<ApiConfig> {
    Ok(ApiConfig {
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
    })
}

pub(super) fn load_log_config() -> LogConfig {
    LogConfig {
        level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        file: env::var("LOG_FILE").ok(),
    }
}

pub(super) fn load_workflow_config() -> WorkflowConfig {
    WorkflowConfig {
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
    }
}

pub(super) fn get_cache_config() -> CacheConfig {
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

pub(super) fn get_queue_config() -> Result<QueueConfig> {
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

pub(super) fn get_mail_config() -> MailConfig {
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

pub(super) fn get_license_config() -> LicenseConfig {
    LicenseConfig {
        license_key: env::var("LICENSE_KEY").ok(),
        private_key: env::var("LICENSE_PRIVATE_KEY").ok(),
        public_key: env::var("LICENSE_PUBLIC_KEY").ok(),
        verification_url: LicenseConfig::default().verification_url,
        statistics_url: LicenseConfig::default().statistics_url,
    }
}

pub(super) fn load_required_cron(name: &str) -> Result<String> {
    let cron =
        env::var(name).map_err(|_| crate::error::Error::Config(format!("{name} not set")))?;
    utils::validate_cron(&cron)
        .map_err(|e| crate::error::Error::Config(format!("Invalid {name} '{cron}': {e}")))?;
    Ok(cron)
}

pub(super) fn load_retention_days<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr + ToString + Copy,
{
    env::var(name)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|_| crate::error::Error::Config(format!("{name} must be a valid number")))
}
