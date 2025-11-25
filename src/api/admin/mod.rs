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
                    .configure(r_data_core_api::admin::entity_definitions::register_routes),
            )
            .service(web::scope("/workflows").configure(r_data_core_api::admin::workflows::register_routes))
            .service(web::scope("/dsl").configure(r_data_core_api::admin::dsl::register_routes))
            .service(web::scope("/api-keys").configure(r_data_core_api::admin::api_keys::register_routes))
            .service(web::scope("/permissions").configure(r_data_core_api::admin::permissions::register_routes))
            .service(web::scope("/system").configure(r_data_core_api::admin::system::register_routes)),
    );
}
