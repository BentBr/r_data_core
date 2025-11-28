#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;

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
}
