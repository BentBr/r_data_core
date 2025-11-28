#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::web;

pub mod dynamic_entities;
pub mod entities;
pub mod queries;
pub mod workflows;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .configure(entities::register_routes)
            .configure(queries::register_routes)
            .configure(workflows::register_routes) // Register workflows BEFORE dynamic_entities to avoid route conflicts
            .configure(dynamic_entities::register_routes),
    );
}
