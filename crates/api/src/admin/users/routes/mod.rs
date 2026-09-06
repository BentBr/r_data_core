#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod read;
pub mod roles;
pub mod write;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use read::*;
pub use roles::*;
pub use write::*;

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
