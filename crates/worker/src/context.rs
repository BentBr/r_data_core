#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::maintenance::task::TaskContext as TaskContextTrait;
use sqlx::PgPool;

/// Context provided to maintenance tasks for execution
#[derive(Clone)]
pub struct TaskContext {
    /// Database connection pool
    pool: PgPool,
    /// Cache manager (optional, for tasks that need caching)
    #[allow(dead_code)] // Will be used by tasks that need caching
    cache: Option<Arc<CacheManager>>,
}

impl TaskContext {
    /// Create a new task context with a database pool
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // PgPool is not const-constructible
    pub fn new(pool: PgPool) -> Self {
        Self { pool, cache: None }
    }

    /// Create a new task context with a database pool and cache manager
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // PgPool and Arc are not const-constructible
    pub fn with_cache(pool: PgPool, cache: Arc<CacheManager>) -> Self {
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
