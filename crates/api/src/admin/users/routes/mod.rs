#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod crud;
pub mod roles;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use crud::*;
pub use roles::*;

/// Register user routes
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(list_users)
        .service(get_user)
        .service(create_user)
        .service(update_user)
        .service(delete_user)
        .service(get_user_roles)
        .service(assign_roles_to_user);
}
