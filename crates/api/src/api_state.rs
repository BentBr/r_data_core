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
