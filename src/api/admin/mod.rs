pub mod api_keys;
pub mod auth;
pub mod dsl;
pub mod entity_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

use actix_web::web;

/// Register all admin API routes
/// 
/// Route handlers remain in the main crate as they need access to concrete ApiState
/// and service types. Models are in the API crate (r_data_core_api::admin::*).
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .configure(r_data_core_api::admin::auth::register_routes)
            // Module routes
            .service(
                web::scope("/entity-definitions")
                    .configure(entity_definitions::routes::register_routes),
            )
            .service(web::scope("/workflows").configure(workflows::routes::register_routes))
            .service(web::scope("/dsl").configure(dsl::routes::register_routes))
            .service(web::scope("/api-keys").configure(r_data_core_api::admin::api_keys::register_routes))
            .service(web::scope("/permissions").configure(permissions::routes::register_routes))
            .service(web::scope("/system").configure(r_data_core_api::admin::system::register_routes)),
    );
}
