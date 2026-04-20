#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};
use time::{Duration, OffsetDateTime};

use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_persistence::OutboxRepository;

/// Maintenance task that removes terminal outbox rows after a retention period.
pub struct OutboxPurgerTask {
    cron: String,
    retention_days: u32,
}

impl OutboxPurgerTask {
    /// Create a new `OutboxPurgerTask`.
    #[must_use]
    pub fn new(cron: String, retention_days: u32) -> Self {
        Self {
            cron,
            retention_days,
        }
    }
}

#[async_trait]
impl MaintenanceTask for OutboxPurgerTask {
    fn name(&self) -> &'static str {
        "outbox_purger"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "[outbox_purger] Starting outbox cleanup task with retention {} day(s)",
            self.retention_days
        );

        let pool = context.pool();
        let repo = OutboxRepository::new(pool.clone());
        let cutoff = OffsetDateTime::now_utc() - Duration::days(i64::from(self.retention_days));

        match repo.purge_terminal_older_than(cutoff).await {
            Ok(count) => {
                info!("[outbox_purger] Deleted {count} terminal outbox row(s)");
            }
            Err(e) => {
                warn!("[outbox_purger] Failed to purge outbox rows: {e}");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }

        info!("[outbox_purger] Outbox cleanup task completed successfully");
        Ok(())
    }
}
