pub mod admin;
pub mod auth;
pub mod docs;
pub mod middleware;
pub mod public;
pub mod response;

use actix_web::{web, App, HttpResponse, Result, ResponseError};
use actix_web::middleware as web_middleware;
use sqlx::PgPool;
use std::sync::Arc;

use crate::cache::CacheManager;
pub use crate::api::response::{ApiResponse, ApiError, Status};

/// Shared application state
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// Cache manager
    pub cache_manager: Arc<CacheManager>,
}

// 404 handler
async fn not_found_handler() -> Result<HttpResponse, ApiError> {
    Err(ApiError::NotFound("Resource not found".to_string()))
}

// Configuration struct for API routes
pub struct ApiConfiguration {
    pub enable_auth: bool,
    pub enable_admin: bool,
    pub enable_public: bool,
    pub enable_docs: bool,
}

impl Default for ApiConfiguration {
    fn default() -> Self {
        Self {
            enable_auth: true,
            enable_admin: true,
            enable_public: true,
            enable_docs: true,
        }
    }
}

// Add global error and 404 handlers to app config
pub fn configure_app(cfg: &mut web::ServiceConfig) {
    configure_app_with_options(cfg, ApiConfiguration::default());
}

// Configure app with customizable options
pub fn configure_app_with_options(cfg: &mut web::ServiceConfig, options: ApiConfiguration) {
    let mut scope = web::scope("");
    
    if options.enable_auth {
        log::debug!("Registering auth routes");
        scope = scope.configure(auth::register_routes);
    }
    
    if options.enable_admin {
        log::debug!("Registering admin routes");
        scope = scope.configure(admin::register_routes);
    }
    
    if options.enable_public {
        log::debug!("Registering public routes");
        scope = scope.configure(public::register_routes);
    }
    
    if options.enable_docs {
        log::debug!("Registering documentation routes");
        scope = scope.configure(docs::register_routes);
    } else {
        log::warn!("Documentation routes are DISABLED");
    }
    
    cfg.service(scope.default_service(web::route().to(not_found_handler)));
    log::debug!("All routes registered");
}
