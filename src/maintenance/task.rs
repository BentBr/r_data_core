use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::cache::CacheManager;
use crate::error::Result;

/// Context provided to maintenance tasks for execution
pub struct TaskContext {
    /// Database connection pool
    pub pool: PgPool,
    /// Cache manager
    pub cache: Arc<CacheManager>,
}

/// Trait that all maintenance tasks must implement
#[async_trait]
pub trait MaintenanceTask: Send + Sync {
    /// Unique identifier for this task
    fn name(&self) -> &'static str;

    /// Cron expression for scheduling this task
    fn cron(&self) -> &str;

    /// Execute the maintenance task
    async fn execute(&self, context: &TaskContext) -> Result<()>;
}
