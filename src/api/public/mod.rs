use actix_web::web;

// Public routes moved to r_data_core_api::public
pub mod dynamic_entities; // Still in main crate - needs validator functions

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(r_data_core_api::public::entities::register_routes)
            .configure(r_data_core_api::public::queries::register_routes)
            .configure(r_data_core_api::public::workflows::register_routes) // Register workflows BEFORE dynamic_entities to avoid route conflicts
            .configure(dynamic_entities::register_routes),
    );
}
