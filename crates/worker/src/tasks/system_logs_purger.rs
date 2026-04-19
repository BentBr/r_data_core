#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};

use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_persistence::{SystemLogRepository, SystemLogRepositoryTrait};

/// Maintenance task that purges old system log entries
pub struct SystemLogsPurgerTask {
    cron: String,
    retention_days: u64,
}

impl SystemLogsPurgerTask {
    /// Create a new `SystemLogsPurgerTask` with the given cron expression and retention period
    ///
    /// # Arguments
    /// * `cron` - Cron expression for scheduling this task
    /// * `retention_days` - Number of days to retain system logs
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(cron: String, retention_days: u64) -> Self {
        Self {
            cron,
            retention_days,
        }
    }
}

#[async_trait]
impl MaintenanceTask for SystemLogsPurgerTask {
    fn name(&self) -> &'static str {
        "system_logs_purger"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let retention_days = self.retention_days;
        info!(
            "[system_logs_purger] Starting system logs purging task (retention: {retention_days} days)"
        );

        let pool = context.pool();
        let repo = SystemLogRepository::new(pool.clone());
        let days = i64::try_from(retention_days).unwrap_or(90);

        match repo.delete_older_than_days(days).await {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[system_logs_purger] Purged {count} system log entries older than {retention_days} days"
                    );
                } else {
                    info!("[system_logs_purger] No system log entries to purge");
                }
            }
            Err(e) => {
                warn!("[system_logs_purger] Failed to purge system logs: {e}");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }

        info!("[system_logs_purger] System logs purging task completed successfully");
        Ok(())
    }
}
