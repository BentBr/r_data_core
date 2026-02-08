#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

use crate::config::{
    ApiConfig, CacheConfig, DatabaseConfig, LicenseConfig, LogConfig, QueueConfig,
};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application environment (development, production, etc.)
    pub environment: String,

    /// Database configuration
    pub database: DatabaseConfig,

    /// API configuration
    pub api: ApiConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Log configuration
    pub log: LogConfig,
    /// Queue configuration (mandatory)
    pub queue: QueueConfig,
    /// License configuration
    pub license: LicenseConfig,
}

/// Worker-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Interval in seconds to reconcile scheduled jobs with DB
    pub job_queue_update_interval_secs: u64,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Workflow configuration
    pub workflow: crate::config::WorkflowConfig,
    /// Queue configuration (required for worker)
    pub queue: QueueConfig,
    /// Cache configuration (optional, uses same Redis as queue if available)
    pub cache: CacheConfig,
    /// License configuration
    pub license: LicenseConfig,
}

/// Maintenance worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    /// Cron expression for the version purger task (required)
    pub version_purger_cron: String,

    /// Cron expression for the refresh token cleanup task (required)
    pub refresh_token_cleanup_cron: String,

    /// Cron expression for workflow run logs purger task (required)
    pub workflow_run_logs_purger_cron: String,

    /// Database configuration used by the maintenance worker
    pub database: DatabaseConfig,

    /// Cache configuration
    pub cache: CacheConfig,
    /// Redis URL for cache usage (mandatory)
    pub redis_url: String,
    /// License configuration
    pub license: LicenseConfig,
    /// API configuration (for admin URI and CORS origins)
    pub api: ApiConfig,
}
