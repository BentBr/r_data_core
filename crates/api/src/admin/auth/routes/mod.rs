#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod helpers;
pub mod login;
pub mod oidc;
pub mod password_reset;
pub mod session;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use login::*;
pub use oidc::*;
pub use password_reset::*;
pub use session::*;

/// Register auth routes
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(admin_login)
        .service(admin_register)
        .service(admin_logout)
        .service(admin_refresh_token)
        .service(admin_revoke_all_tokens)
        .service(get_user_permissions)
        .service(forgot_password)
        .service(reset_password)
        .service(oidc_start)
        .service(oidc_callback)
        .service(oidc_exchange);
}
