#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use dotenvy::dotenv;
use std::env;

use crate::config::{
    ApiConfig, AppConfig, CacheConfig, DatabaseConfig, LogConfig, MaintenanceConfig, QueueConfig,
    WorkerConfig, WorkflowConfig,
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
    };

    let log = LogConfig {
        level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        file: env::var("LOG_FILE").ok(),
    };

    let cache = get_cache_config();
    let queue = get_queue_config()?;

    Ok(AppConfig {
        environment,
        database,
        api,
        cache,
        log,
        queue,
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

    Ok(WorkerConfig {
        job_queue_update_interval_secs,
        database,
        workflow,
        queue,
        cache,
    })
}

/// Load maintenance configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_maintenance_config() -> Result<MaintenanceConfig> {
    // Ensure .env is loaded for binaries that only use MaintenanceConfig
    dotenv().ok();

    let cron = env::var("MAINTENANCE_CRON")
        .map_err(|_| crate::error::Error::Config("MAINTENANCE_CRON not set".to_string()))?;
    // Validate cron expression using the same logic as cron preview
    utils::validate_cron(&cron).map_err(|e| {
        crate::error::Error::Config(format!("Invalid MAINTENANCE_CRON '{cron}': {e}"))
    })?;

    let version_purger_cron = env::var("VERSION_PURGER_CRON")
        .map_err(|_| crate::error::Error::Config("VERSION_PURGER_CRON not set".to_string()))?;
    utils::validate_cron(&version_purger_cron).map_err(|e| {
        crate::error::Error::Config(format!(
            "Invalid VERSION_PURGER_CRON '{version_purger_cron}': {e}",
        ))
    })?;

    // Prefer dedicated MAINTENANCE_*, then WORKER_*, then general DATABASE_* where sensible
    let connection_string = env::var("MAINTENANCE_DATABASE_URL")
        .map_err(|_| crate::error::Error::Config("MAINTENANCE_DATABASE_URL not set".to_string()))?;

    let max_connections: u32 = env::var("MAINTENANCE_DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    let connection_timeout: u64 = env::var("MAINTENANCE_DATABASE_CONNECTION_TIMEOUT")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30);

    let database = DatabaseConfig {
        connection_string,
        max_connections,
        connection_timeout,
    };

    let cache = get_cache_config();
    let redis_url = env::var("REDIS_URL")
        .map_err(|_| crate::error::Error::Config("REDIS_URL not set".to_string()))?;

    Ok(MaintenanceConfig {
        cron,
        version_purger_cron,
        database,
        cache,
        redis_url,
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
    };

    Ok(config)
}
