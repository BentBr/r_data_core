pub mod api_keys;
pub mod auth;
pub mod class_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

use crate::api::middleware::JwtAuth;
use actix_web::web;

/// Register all admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // Routes that don't require authentication
    cfg.service(
        web::scope("/admin/api/v1")
            .service(auth::admin_login)
            .service(auth::admin_register)
            .service(
                web::scope("")
                    .wrap(JwtAuth::new())
                    // Auth routes that require authentication
                    .service(auth::admin_logout)
                    // Other admin modules
                    .configure(class_definitions::register_routes)
                    .configure(workflows::register_routes)
                    .configure(permissions::register_routes)
                    .configure(system::register_routes)
                    .configure(api_keys::register_routes),
            ),
    );
}
