pub mod api_keys;
pub mod class_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

use crate::api::middleware::JwtAuth;
use actix_web::web;

// Re-export PaginationQuery and PathUuid so they can still be referenced from other modules
pub use class_definitions::models::PaginationQuery;
pub use class_definitions::models::PathUuid;

/// Register all admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .wrap(JwtAuth::new())
            .configure(class_definitions::register_routes)
            .configure(workflows::register_routes)
            .configure(permissions::register_routes)
            .configure(system::register_routes)
            .configure(api_keys::register_routes),
    );
}
