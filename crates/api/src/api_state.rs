use actix_web::web;
use sqlx::PgPool;

use crate::response::ApiResponse;
use crate::health;
use crate::middleware;

pub trait ApiStateTrait: Send + Sync + 'static {
    fn db_pool(&self) -> &PgPool;
    fn jwt_secret(&self) -> &str;
    fn api_key_service_ref(&self) -> &dyn std::any::Any;
    fn permission_scheme_service_ref(&self) -> &dyn std::any::Any;
    fn api_config_ref(&self) -> &dyn std::any::Any;
    fn entity_definition_service_ref(&self) -> &dyn std::any::Any;
    fn cache_manager_ref(&self) -> &dyn std::any::Any;
    fn workflow_service_ref(&self) -> &dyn std::any::Any;
    fn queue_ref(&self) -> &dyn std::any::Any;
    
    /// Get API config - helper method that downcasts from api_config_ref
    fn api_config(&self) -> &r_data_core_core::config::ApiConfig {
        self.api_config_ref()
            .downcast_ref::<r_data_core_core::config::ApiConfig>()
            .expect("ApiState must provide ApiConfig")
    }
    
    /// Get permission scheme service - helper method that downcasts from permission_scheme_service_ref
    fn permission_scheme_service(&self) -> &r_data_core_services::PermissionSchemeService {
        self.permission_scheme_service_ref()
            .downcast_ref::<r_data_core_services::PermissionSchemeService>()
            .expect("ApiState must provide PermissionSchemeService")
    }
    
    /// Get entity definition service - helper method that downcasts from entity_definition_service_ref
    fn entity_definition_service(&self) -> &r_data_core_services::EntityDefinitionService {
        self.entity_definition_service_ref()
            .downcast_ref::<r_data_core_services::EntityDefinitionService>()
            .expect("ApiState must provide EntityDefinitionService")
    }
    
    /// Get cache manager - helper method that downcasts from cache_manager_ref
    fn cache_manager(&self) -> &std::sync::Arc<r_data_core_core::cache::CacheManager> {
        self.cache_manager_ref()
            .downcast_ref::<std::sync::Arc<r_data_core_core::cache::CacheManager>>()
            .expect("ApiState must provide CacheManager")
    }
    
    /// Get workflow service - helper method that downcasts from workflow_service_ref
    fn workflow_service(&self) -> &r_data_core_services::WorkflowService {
        self.workflow_service_ref()
            .downcast_ref::<r_data_core_services::WorkflowService>()
            .expect("ApiState must provide WorkflowService")
    }
    
    /// Get queue - helper method that downcasts from queue_ref
    fn queue(&self) -> &std::sync::Arc<r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue> {
        self.queue_ref()
            .downcast_ref::<std::sync::Arc<r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue>>()
            .expect("ApiState must provide ApalisRedisQueue")
    }
}

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

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    configure_app_with_options(cfg, ApiConfiguration::default());
}

pub fn configure_app_with_options(cfg: &mut web::ServiceConfig, _options: ApiConfiguration) {
    cfg.service(health::admin_health_check)
        .service(health::public_health_check);

    let scope = web::scope("").wrap(middleware::ErrorHandler);

    async fn not_found_handler() -> impl actix_web::Responder {
        ApiResponse::<()>::not_found("API resource not found")
    }
    let scope = scope.default_service(web::route().to(not_found_handler));

    cfg.service(scope);
}
