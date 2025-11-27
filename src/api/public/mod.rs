use actix_web::web;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(r_data_core_api::public::entities::register_routes)
            .configure(r_data_core_api::public::queries::register_routes)
            .configure(r_data_core_api::public::workflows::register_routes) // Register workflows BEFORE dynamic_entities to avoid route conflicts
            .configure(r_data_core_api::public::dynamic_entities::register_routes),
    );
}
