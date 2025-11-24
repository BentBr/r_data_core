#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_core::maintenance::TaskContext as TaskContextTrait;
use sqlx::PgPool;

/// Context provided to maintenance tasks for execution
pub struct TaskContext {
    /// Database connection pool
    pool: PgPool,
    /// Cache manager (optional, for tasks that need caching)
    #[allow(dead_code)] // Will be used by tasks that need caching
    cache: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl TaskContext {
    /// Create a new task context with a database pool
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: None,
        }
    }

    /// Create a new task context with a database pool and cache manager
    #[must_use]
    pub fn with_cache(
        pool: PgPool,
        cache: Arc<dyn std::any::Any + Send + Sync>,
    ) -> Self {
        Self {
            pool,
            cache: Some(cache),
        }
    }
}

impl TaskContextTrait for TaskContext {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
}

