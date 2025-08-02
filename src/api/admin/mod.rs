pub mod api_keys;
pub mod auth;
pub mod entity_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

use actix_web::web;

/// Register all admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .service(auth::admin_login)
            .service(auth::admin_register)
            .service(auth::admin_logout)
            .service(auth::admin_refresh_token)
            .service(auth::admin_revoke_all_tokens)
            // Module routes
            .service(
                web::scope("/entity-definitions")
                    .configure(entity_definitions::routes::register_routes),
            )
            .service(web::scope("/workflows").configure(workflows::routes::register_routes))
            .service(web::scope("/api-keys").configure(api_keys::routes::register_routes))
            .service(web::scope("/permissions").configure(permissions::routes::register_routes))
            .service(web::scope("/system").configure(system::routes::register_routes)),
    );
}
