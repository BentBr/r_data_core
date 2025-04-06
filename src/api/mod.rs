pub mod auth;
pub mod admin;
pub mod public;
pub mod middleware;
pub mod docs;

use sqlx::PgPool;
use std::sync::Arc;
use crate::cache::CacheManager;

/// Shared state for API handlers
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,
    
    /// JWT secret for authentication
    pub jwt_secret: String,
    
    /// Cache manager
    pub cache_manager: Arc<CacheManager>,
} 