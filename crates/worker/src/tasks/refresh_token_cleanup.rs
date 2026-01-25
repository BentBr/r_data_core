#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};
use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};

/// Maintenance task that cleans up expired and revoked refresh tokens
pub struct RefreshTokenCleanupTask {
    cron: String,
}

impl RefreshTokenCleanupTask {
    /// Create a new `RefreshTokenCleanupTask` with the given cron expression
    ///
    /// # Arguments
    /// * `refresh_token_cleanup_cron` - Cron expression for scheduling this task
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(refresh_token_cleanup_cron: String) -> Self {
        Self {
            cron: refresh_token_cleanup_cron,
        }
    }
}

#[async_trait]
impl MaintenanceTask for RefreshTokenCleanupTask {
    fn name(&self) -> &'static str {
        "refresh_token_cleanup"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[refresh_token_cleanup] Starting refresh token cleanup task");

        let pool = context.pool();
        let repo = RefreshTokenRepository::new(pool.clone());

        // Clean up expired and revoked tokens
        match repo.cleanup_expired_tokens().await {
            Ok(count) => {
                info!(
                    "[refresh_token_cleanup] Cleaned up {count} expired and revoked refresh tokens"
                );
            }
            Err(e) => {
                warn!("[refresh_token_cleanup] Failed to cleanup refresh tokens: {e}");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }

        info!("[refresh_token_cleanup] Refresh token cleanup task completed successfully");
        Ok(())
    }
}
