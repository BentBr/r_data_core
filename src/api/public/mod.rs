use actix_web::web;

pub mod entities;
pub mod queries;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // Removed middleware authentication - now using CombinedRequiredAuth extractor pattern
    // like the admin routes, which works with both JWT and API key auth
    cfg.service(
        web::scope("/api/v1")
            .configure(entities::register_routes)
            .configure(queries::register_routes),
    );
}
