pub mod models;
pub mod repository;
pub mod routes;

use crate::api::middleware::JwtAuth;
use actix_web::web;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // Apply JWT authentication middleware to all class definition routes
    cfg.service(
        web::scope("")
            .wrap(JwtAuth::new())
            .service(routes::list_class_definitions)
            .service(routes::get_class_definition)
            .service(routes::create_class_definition)
            .service(routes::update_class_definition)
            .service(routes::delete_class_definition)
            .service(routes::apply_class_definition_schema),
    );
}
