#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod read;
pub mod roles;
pub mod write;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use read::*;
pub use roles::*;
pub use write::*;

/// Refuse a stored-role assignment for an SSO account.
///
/// An SSO account's roles are re-derived from its token on every sign-in and
/// never stored (docs/SSO.md, "Roles are not stored"). Accepting them would
/// create a second, silent authority that group removal at the provider could
/// never narrow, so every path that writes `user_roles` asks this first.
#[must_use]
pub fn refuse_sso_role_assignment(
    user: &r_data_core_core::admin_user::AdminUser,
) -> Option<actix_web::HttpResponse> {
    user.is_sso_provisioned.then(|| {
        crate::response::ApiResponse::<()>::bad_request(
            "This user signs in through single sign-on; roles come from the identity \
             provider's groups and cannot be assigned here.",
        )
    })
}

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
