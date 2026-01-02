#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;
use std::future::Future;
use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::MaintenanceConfig;
use tokio_cron_scheduler::JobScheduler;

/// Trait for registering maintenance tasks with the scheduler
pub trait TaskRegistrar: Send + Sync {
    /// Register the task with the scheduler
    ///
    /// # Errors
    /// Returns an error if task registration fails
    fn register(
        &self,
        scheduler: &JobScheduler,
        pool: PgPool,
        cache_manager: Arc<CacheManager>,
        config: &MaintenanceConfig,
    ) -> impl Future<Output = r_data_core_core::error::Result<()>> + Send;
}
