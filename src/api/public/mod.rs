use actix_web::web;

pub mod dynamic_entities;
pub mod entities;
pub mod queries;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(entities::routes::register_routes)
            .configure(queries::register_routes)
            .configure(dynamic_entities::register_routes),
    );
}
