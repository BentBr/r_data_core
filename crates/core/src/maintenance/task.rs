#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use std::sync::Arc;

use crate::cache::CacheManager;

/// Trait that all maintenance tasks must implement
#[async_trait]
pub trait MaintenanceTask: Send + Sync {
    /// Unique identifier for this task
    fn name(&self) -> &'static str;

    /// Cron expression for scheduling this task
    fn cron(&self) -> &str;

    /// Execute the maintenance task
    ///
    /// # Errors
    /// Returns an error if task execution fails
    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Context provided to maintenance tasks for execution
///
/// This trait allows tasks to access resources without depending on specific implementations.
pub trait TaskContext: Send + Sync {
    /// Get the database connection pool
    ///
    /// # Returns
    /// A reference to the `PostgresSQL` connection pool
    fn pool(&self) -> &sqlx::PgPool;

    /// Get the cache manager if available
    ///
    /// # Returns
    /// An optional reference to the shared cache manager
    fn cache_manager(&self) -> Option<Arc<CacheManager>>;

    /// Get the cache manager, falling back to a minimal in-memory instance
    ///
    /// This is useful for maintenance tasks that need a `CacheManager` for settings
    /// lookup but can function without a shared Redis-backed cache.
    fn cache_manager_or_default(&self) -> Arc<CacheManager> {
        self.cache_manager().unwrap_or_else(|| {
            Arc::new(CacheManager::new(crate::config::CacheConfig {
                entity_definition_ttl: 3600,
                api_key_ttl: 600,
                enabled: true,
                ttl: 3600,
                max_size: 10000,
            }))
        })
    }
}
