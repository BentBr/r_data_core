#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use log::info;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::LicenseConfig;
use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_services::StatisticsService;

/// Maintenance task that collects and sends statistics daily with randomized timing
pub struct StatisticsCollectionTask {
    cron: String,
    config: LicenseConfig,
    admin_uri: String,
    cors_origins: Vec<String>,
    /// Database URL used as fallback seed for scheduling when no license key is present
    database_url: String,
}

impl StatisticsCollectionTask {
    /// Create a new `StatisticsCollectionTask` with the given cron expression
    ///
    /// # Arguments
    /// * `cron` - Cron expression for scheduling this task
    /// * `config` - License configuration
    /// * `admin_uri` - Admin URI
    /// * `cors_origins` - CORS origins
    /// * `database_url` - Database URL (used as fallback seed for scheduling)
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(
        cron: String,
        config: LicenseConfig,
        admin_uri: String,
        cors_origins: Vec<String>,
        database_url: String,
    ) -> Self {
        Self {
            cron,
            config,
            admin_uri,
            cors_origins,
            database_url,
        }
    }

    /// Calculate deterministic hour and minute based on license key ID or database URL
    /// Returns (hour, minute) tuple
    ///
    /// Priority:
    /// 1. License key ID (if available) - unique per customer
    /// 2. Database URL (fallback) - unique per instance
    fn calculate_schedule(&self) -> (u8, u8) {
        // Determine the seed: prefer license_id, fallback to database_url
        let seed = self.config.license_key.as_ref().map_or_else(
            || self.database_url.clone(), // Fallback: use database URL as seed (stable per instance)
            |license_key| {
                // Try to extract license_id from JWT, or use the full key
                Self::extract_license_id(license_key).unwrap_or_else(|_| license_key.clone())
            },
        );

        // Hash the seed using SHA256
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let hash = hasher.finalize();

        // Take first byte for hour (0-23), second byte for minute (0-59)
        let hour = hash[0] % 24;
        let minute = hash[1] % 60;
        (hour, minute)
    }

    /// Extract `license_id` from JWT token
    fn extract_license_id(
        license_key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let parts: Vec<&str> = license_key.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }

        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload)?;
        let claims: serde_json::Value = serde_json::from_slice(&decoded)?;

        claims
            .get("license_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| "Missing license_id in JWT claims".into())
    }

    /// Check if task should run (not run within last hour)
    async fn should_run(&self, cache: &CacheManager) -> bool {
        let cache_key = "task:statistics:last_run";

        if let Ok(Some(last_run_timestamp)) = cache.get::<i64>(cache_key).await {
            if let Some(last_run) = DateTime::from_timestamp(last_run_timestamp, 0) {
                let now = Utc::now();
                let hours_since_last_run = (now - last_run).num_hours();

                if hours_since_last_run < 1 {
                    info!(
                        "[statistics] Skipping - last run was {} minutes ago",
                        (now - last_run).num_minutes()
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Update last run timestamp in cache
    async fn update_last_run(&self, cache: &CacheManager) {
        let cache_key = "task:statistics:last_run";
        let now = Utc::now().timestamp();
        let ttl = Some(7200_u64); // 2 hours TTL to cover hour boundaries

        if let Err(e) = cache.set(cache_key, &now, ttl).await {
            // Silent failure - only print to stdout
            println!("[statistics] Failed to update last run timestamp: {e}");
        }
    }
}

#[async_trait]
impl MaintenanceTask for StatisticsCollectionTask {
    fn name(&self) -> &'static str {
        "statistics_collection"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Calculate deterministic schedule from license key or database URL
        let (target_hour, target_minute) = self.calculate_schedule();

        // Get cache manager
        let cache_manager: Arc<CacheManager> = context.cache_manager().ok_or_else(|| {
            Box::new(std::io::Error::other("Cache manager not available"))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Determine if we should run based on deterministic hour and minute
        let now = Utc::now();
        let current_hour = u8::try_from(now.hour()).unwrap_or(0);
        let current_minute = u8::try_from(now.minute()).unwrap_or(0);

        // Check if we're in the target hour and at or past the target minute
        // Using >= for minute allows the task to run even if the exact minute was missed
        // (e.g., due to system load). The should_run() cache check prevents multiple runs.
        let in_target_window = current_hour == target_hour && current_minute >= target_minute;

        if !in_target_window {
            // Only log occasionally to avoid spam (every 15 minutes during wrong hours)
            if current_minute % 15 == 0 {
                info!(
                    "[statistics] Waiting - current time {current_hour:02}:{current_minute:02}, target window starts at {target_hour:02}:{target_minute:02}"
                );
            }
            return Ok(());
        }

        // Check if we should run (prevent multiple runs in same hour)
        if !self.should_run(&cache_manager).await {
            return Ok(());
        }

        // Create statistics repository and service
        let repository = Arc::new(r_data_core_persistence::StatisticsRepository::new(
            context.pool().clone(),
        ));
        // Use database_url for generating deterministic instance key for unlicensed instances
        let statistics_service = StatisticsService::with_database_url(
            self.config.clone(),
            repository,
            self.database_url.clone(),
        );

        // Collect and send statistics (silent failure - only stdout)
        statistics_service
            .collect_and_send(&self.admin_uri, &self.cors_origins)
            .await;

        // Update last run timestamp
        self.update_last_run(&cache_manager).await;

        Ok(())
    }
}
