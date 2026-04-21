#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};

use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_persistence::{PasswordResetRepository, PasswordResetRepositoryTrait};

/// Maintenance task that cleans up expired password reset tokens
pub struct PasswordResetCleanupTask {
    cron: String,
}

impl PasswordResetCleanupTask {
    /// Create a new `PasswordResetCleanupTask` with the given cron expression
    ///
    /// # Arguments
    /// * `cron` - Cron expression for scheduling this task
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(cron: String) -> Self {
        Self { cron }
    }
}

#[async_trait]
impl MaintenanceTask for PasswordResetCleanupTask {
    fn name(&self) -> &'static str {
        "password_reset_cleanup"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[password_reset_cleanup] Starting password reset token cleanup task");

        let pool = context.pool();
        let repo = PasswordResetRepository::new(pool.clone());

        match repo.delete_expired().await {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "[password_reset_cleanup] Cleaned up {count} expired password reset tokens"
                    );
                } else {
                    info!("[password_reset_cleanup] No expired password reset tokens to clean up");
                }
            }
            Err(e) => {
                warn!("[password_reset_cleanup] Failed to cleanup password reset tokens: {e}");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }

        info!("[password_reset_cleanup] Password reset token cleanup task completed successfully");
        Ok(())
    }
}
