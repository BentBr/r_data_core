#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api_keys;
pub mod auth;
pub mod dsl;
pub mod entity_definitions;
pub mod meta;
pub mod permissions;
pub mod query_helpers;
pub mod system;
pub mod users;
pub mod workflows;

use actix_web::web;

/// Register all admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .configure(auth::register_routes)
            // Module routes
            .service(
                web::scope("/entity-definitions").configure(entity_definitions::register_routes),
            )
            .service(web::scope("/workflows").configure(workflows::register_routes))
            .service(web::scope("/dsl").configure(dsl::register_routes))
            .service(web::scope("/api-keys").configure(api_keys::register_routes))
            .service(web::scope("/roles").configure(permissions::register_routes))
            .service(web::scope("/users").configure(users::register_routes))
            .service(web::scope("/system").configure(system::register_routes))
            .service(web::scope("/meta").configure(meta::register_routes)),
    );
}
