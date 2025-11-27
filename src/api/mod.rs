// Admin routes moved to r_data_core_api::admin
pub mod docs;
// Health routes moved to r_data_core_api::health
pub mod public;

use actix_web::{get, web, Responder};
use sqlx::PgPool;
use std::sync::Arc;

pub use r_data_core_api::response::ApiResponse;
use r_data_core_core::cache::CacheManager;
use r_data_core_services::{AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService, PermissionSchemeService, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

/// Shared application state
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,

    /// API configuration (includes JWT secret and expiration)
    pub api_config: r_data_core_core::config::ApiConfig,

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

    /// Permission scheme service
    pub permission_scheme_service: PermissionSchemeService,

    /// Queue client for producing jobs
    pub queue: Arc<ApalisRedisQueue>,
}

// Implement ApiStateTrait for ApiState to allow API crate routes to use it
impl r_data_core_api::api_state::ApiStateTrait for ApiState {
    fn db_pool(&self) -> &PgPool {
        &self.db_pool
    }

    fn jwt_secret(&self) -> &str {
        &self.api_config.jwt_secret
    }

    fn api_key_service_ref(&self) -> &dyn std::any::Any {
        &self.api_key_service
    }

    fn permission_scheme_service_ref(&self) -> &dyn std::any::Any {
        &self.permission_scheme_service
    }

    fn api_config_ref(&self) -> &dyn std::any::Any {
        &self.api_config
    }

    fn entity_definition_service_ref(&self) -> &dyn std::any::Any {
        &self.entity_definition_service
    }

    fn dynamic_entity_service_ref(&self) -> Option<&dyn std::any::Any> {
        self.dynamic_entity_service.as_ref().map(|s| s as &dyn std::any::Any)
    }

    fn cache_manager_ref(&self) -> &dyn std::any::Any {
        &*self.cache_manager
    }

    fn workflow_service_ref(&self) -> &dyn std::any::Any {
        &self.workflow_service
    }

    fn queue_ref(&self) -> &dyn std::any::Any {
        &*self.queue
    }
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
    cfg.service(r_data_core_api::health::admin_health_check)
        .service(r_data_core_api::health::public_health_check);

    let mut scope = web::scope("").wrap(r_data_core_api::middleware::ErrorHandler); // Add our error handler middleware

    if options.enable_admin {
        log::debug!("Registering admin routes");
        scope = scope.configure(r_data_core_api::admin::register_routes);
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
