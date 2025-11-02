pub mod admin;
pub mod auth;
pub mod docs;
pub mod health;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod public;
pub mod query;
pub mod response;

use actix_web::{get, web, Responder};
use sqlx::PgPool;
use std::sync::Arc;

pub use crate::api::response::ApiResponse;
use crate::cache::CacheManager;
use crate::services::AdminUserService;
use crate::services::ApiKeyService;
use crate::services::DynamicEntityService;
use crate::services::EntityDefinitionService;
use crate::services::WorkflowService;

/// Shared application state
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// Cache manager
    pub cache_manager: Arc<CacheManager>,

    /// API Key service
    pub api_key_service: ApiKeyService,

    /// Admin User service
    pub admin_user_service: AdminUserService,

    /// Entity Definition service
    pub entity_definition_service: EntityDefinitionService,

    /// Dynamic Entity service
    pub dynamic_entity_service: Option<Arc<DynamicEntityService>>,

    /// Workflow service (data import/export workflows)
    pub workflow_service: WorkflowService,
}

// 404 handler for API routes within scope
async fn not_found_handler() -> impl Responder {
    ApiResponse::<()>::not_found("API resource not found")
}

/// Health check endpoint
#[get("/admin/api/v1/health")]
async fn health_check() -> impl Responder {
    ApiResponse::message("Service is healthy")
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
            enable_auth: false,
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
    // Add health check endpoints
    cfg.service(health::admin_health_check)
        .service(health::public_health_check);

    let mut scope = web::scope("").wrap(middleware::ErrorHandler); // Add our error handler middleware

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

    // Add the default API 404 handler for all scoped routes
    scope = scope.default_service(web::route().to(not_found_handler));

    // Add the scoped service to the config
    cfg.service(scope);

    log::debug!("All routes registered");
}

// Add query export to the public API
pub use query::StandardQuery;
