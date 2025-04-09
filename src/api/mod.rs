pub mod admin;
pub mod auth;
pub mod docs;
pub mod middleware;
pub mod public;

use crate::cache::CacheManager;
use sqlx::PgPool;
use std::sync::Arc;

/// Shared state for API handlers
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// Cache manager
    pub cache_manager: Arc<CacheManager>,
}
