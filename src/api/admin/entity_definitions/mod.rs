pub mod conversions;
pub mod routes;

use actix_web::web;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // Apply JWT authentication middleware to all entity definition routes
    cfg.service(
        web::scope("")
            .service(routes::list_entity_definitions)
            .service(routes::get_entity_definition)
            .service(routes::create_entity_definition)
            .service(routes::update_entity_definition)
            .service(routes::delete_entity_definition)
            .service(routes::apply_entity_definition_schema)
            .service(routes::list_entity_definition_versions)
            .service(routes::get_entity_definition_version),
    );
}
