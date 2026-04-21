#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

use crate::config::{
    ApiConfig, CacheConfig, DatabaseConfig, LicenseConfig, LogConfig, MailConfig, QueueConfig,
};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application environment (development, production, etc.)
    pub environment: String,

    /// Whether the workflow outbox is enabled.
    pub outbox_enabled: bool,

    /// Base delay for workflow outbox retries, in seconds.
    pub outbox_retry_base_delay_secs: i64,

    /// Exponential multiplier for workflow outbox retries.
    pub outbox_retry_multiplier: u64,

    /// Maximum delay for workflow outbox retries, in seconds.
    pub outbox_retry_max_delay_secs: i64,

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
    /// Mail configuration
    pub mail: MailConfig,
    /// Base URL of the frontend application (used for e.g. password-reset links)
    pub frontend_base_url: Option<String>,
    /// Minimum seconds between password-reset requests for the same account
    pub password_reset_throttle_seconds: u64,
}

/// Worker-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Interval in seconds to reconcile scheduled jobs with DB
    pub job_queue_update_interval_secs: u64,

    /// Whether the workflow outbox is enabled.
    pub outbox_enabled: bool,

    /// Stale processing lease for outbox rows, in seconds.
    pub outbox_stale_lease_secs: i64,

    /// Base delay for workflow outbox retries, in seconds.
    pub outbox_retry_base_delay_secs: i64,

    /// Exponential multiplier for workflow outbox retries.
    pub outbox_retry_multiplier: u64,

    /// Maximum delay for workflow outbox retries, in seconds.
    pub outbox_retry_max_delay_secs: i64,

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
    /// Mail configuration
    pub mail: MailConfig,
}

/// Maintenance worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    /// Whether the workflow outbox is enabled.
    pub outbox_enabled: bool,

    /// Cron expression for the version purger task (required)
    pub version_purger_cron: String,

    /// Cron expression for the refresh token cleanup task (required)
    pub refresh_token_cleanup_cron: String,

    /// Cron expression for workflow run logs purger task (required)
    pub workflow_run_logs_purger_cron: String,

    /// Cron expression for the system logs purger task (required)
    pub system_logs_purger_cron: String,

    /// Number of days to retain system logs before purging
    pub system_logs_retention_days: u64,

    /// Cron expression for outbox cleanup task (required when outbox is enabled)
    pub outbox_purger_cron: Option<String>,

    /// Retention window for terminal outbox rows, in days
    pub outbox_retention_days: Option<u32>,

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
