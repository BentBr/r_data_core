use actix_web::web;

use crate::api::middleware::CombinedAuth;
pub mod entities;
pub mod queries;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .wrap(CombinedAuth::new())
            .configure(entities::register_routes)
            .configure(queries::register_routes),
    );
}
