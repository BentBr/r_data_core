pub mod entities;
pub mod queries;

use crate::api::middleware::ApiAuth;
use actix_web::web;

/// Register all public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .wrap(ApiAuth::new())
            .configure(entities::register_routes)
            .configure(queries::register_routes),
    );
}
