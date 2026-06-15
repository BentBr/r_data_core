#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod crud;
pub mod fields_and_versions;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use crud::*;
pub use fields_and_versions::*;

/// Register routes for entity definitions
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_entity_definitions)
        .service(get_entity_definition)
        .service(create_entity_definition)
        .service(update_entity_definition)
        .service(delete_entity_definition)
        .service(apply_entity_definition_schema)
        .service(list_entity_fields_by_type)
        .service(list_entity_definition_versions)
        .service(get_entity_definition_version);
}
