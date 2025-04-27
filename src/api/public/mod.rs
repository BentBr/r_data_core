use actix_web::web;

pub mod entities;
pub mod queries;
pub mod dynamic_entity;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(entities::register_routes)
            .configure(queries::register_routes)
            .configure(dynamic_entity::register_routes),
    );
}
