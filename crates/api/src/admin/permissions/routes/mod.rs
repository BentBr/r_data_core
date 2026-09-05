#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod assignments;
pub mod roles_crud;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use assignments::*;
pub use roles_crud::*;

/// Register role routes
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_roles)
        .service(get_role)
        .service(create_role)
        .service(update_role)
        .service(delete_role)
        .service(assign_roles_to_user)
        .service(assign_roles_to_api_key);
}
