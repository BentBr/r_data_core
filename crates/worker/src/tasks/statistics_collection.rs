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
}

impl StatisticsCollectionTask {
    /// Create a new `StatisticsCollectionTask` with the given cron expression
    ///
    /// # Arguments
    /// * `cron` - Cron expression for scheduling this task
    /// * `config` - License configuration
    /// * `admin_uri` - Admin URI
    /// * `cors_origins` - CORS origins
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(
        cron: String,
        config: LicenseConfig,
        admin_uri: String,
        cors_origins: Vec<String>,
    ) -> Self {
        Self {
            cron,
            config,
            admin_uri,
            cors_origins,
        }
    }

    /// Calculate deterministic hour based on license key ID
    fn calculate_hour(&self) -> Option<u8> {
        let license_key = self.config.license_key.as_ref()?;

        // Try to extract license_id from JWT
        let license_id =
            Self::extract_license_id(license_key).unwrap_or_else(|_| license_key.clone());

        // Hash license_id using SHA256
        let mut hasher = Sha256::new();
        hasher.update(license_id.as_bytes());
        let hash = hasher.finalize();

        // Take first byte and calculate hour (0-23)
        let hour = hash[0] % 24;
        Some(hour)
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
        // Get cache manager
        let cache_manager: Arc<CacheManager> = context.cache_manager().ok_or_else(|| {
            Box::new(std::io::Error::other("Cache manager not available"))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Determine if we should run based on deterministic hour and random minute
        let current_hour = u8::try_from(Utc::now().hour()).unwrap_or(0); // Hours are 0-23, so this should never fail
        let target_hour = self.calculate_hour().unwrap_or_else(|| {
            // Fallback to a random hour if license_id cannot be determined
            (rand::random::<u32>() % 24) as u8
        });

        if current_hour != target_hour {
            info!(
                "[statistics] Skipping - current hour {current_hour} != target hour {target_hour}"
            );
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
        let statistics_service = StatisticsService::new(self.config.clone(), repository);

        // Collect and send statistics (silent failure - only stdout)
        statistics_service
            .collect_and_send(&self.admin_uri, &self.cors_origins)
            .await;

        // Update last run timestamp
        self.update_last_run(&cache_manager).await;

        Ok(())
    }
}
