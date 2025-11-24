pub mod api_keys;
pub mod auth;
pub mod dsl;
pub mod entity_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

use actix_web::web;

/// Register all admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .service(auth::routes::admin_login)
            .service(auth::routes::admin_register)
            .service(auth::routes::admin_logout)
            .service(auth::routes::admin_refresh_token)
            .service(auth::routes::admin_revoke_all_tokens)
            // Module routes
            .service(
                web::scope("/entity-definitions")
                    .configure(entity_definitions::routes::register_routes),
            )
            .service(web::scope("/workflows").configure(workflows::routes::register_routes))
            .service(web::scope("/dsl").configure(dsl::routes::register_routes))
            .service(web::scope("/api-keys").configure(api_keys::routes::register_routes))
            .service(web::scope("/permissions").configure(permissions::routes::register_routes))
            .service(web::scope("/system").configure(system::routes::register_routes)),
    );
}
