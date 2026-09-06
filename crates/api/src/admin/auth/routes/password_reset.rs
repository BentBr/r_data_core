#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

use actix_web::{get, post, web, Responder};
use uuid::Uuid;

use crate::admin::auth::models::{ForgotPasswordRequest, ResetPasswordRequest};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use crate::response::ApiResponse;
use r_data_core_core::system_log::SystemLogStatus;

/// The claims to answer with, whichever way the caller authenticated.
///
/// A JWT already carries them. An API key does not, so the same claims are
/// rebuilt from the key's owner and their roles — through
/// `admin_jwt::build_claims`, the one function that flattens permissions, so
/// the answer is identical to the one a JWT for that user would have given.
/// Rebuilding them by hand here would be a second implementation of
/// authorization, which is how the two drift apart.
async fn resolve_claims(
    data: &ApiStateWrapper,
    auth: &CombinedRequiredAuth,
) -> Option<r_data_core_core::admin_jwt::AuthUserClaims> {
    use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
    use std::sync::Arc;

    if let Some(claims) = &auth.jwt_claims {
        return Some(claims.clone());
    }

    let key = auth.api_key_info.as_ref()?;
    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));
    let user = repo.find_by_uuid(&key.user_uuid).await.ok().flatten()?;
    let roles = crate::admin::auth::routes::helpers::load_user_roles(&user, data, &repo).await;

    r_data_core_core::admin_jwt::build_claims(
        &user,
        &roles,
        r_data_core_core::admin_jwt::ACCESS_TOKEN_EXPIRY_SECONDS,
    )
    .ok()
}

/// Get user's allowed routes and permissions
#[utoipa::path(
    get,
    path = "/admin/api/v1/auth/permissions",
    tag = "admin-auth",
    responses(
        (status = 200, description = "User permissions and allowed routes", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("api_key" = [])
    )
)]
#[get("/auth/permissions")]
pub async fn get_user_permissions(
    data: web::Data<ApiStateWrapper>,
    auth: CombinedRequiredAuth,
) -> impl Responder {
    use r_data_core_services::AuthService;

    // Accepts an API key as well as a JWT. "What may I do?" is a question any
    // authenticated principal should be able to ask, and the MCP server asks
    // it at startup precisely so it can avoid offering tools the caller cannot
    // use. Refusing API keys here made that impossible — the server could only
    // fail, and its failure message blamed the key.
    let Some(claims) = resolve_claims(&data, &auth).await else {
        return ApiResponse::<()>::unauthorized("Authentication required");
    };
    let claims = &claims;

    // Use auth service to get user permissions
    let auth_service = AuthService::new();
    let response = auth_service.get_user_permissions(
        claims.is_super_admin,
        &claims.permissions,
        |namespace, perm_type| {
            crate::auth::permission_check::has_permission(claims, namespace, perm_type, None)
        },
    );

    ApiResponse::ok(response)
}

/// Forgot password endpoint — initiates the password reset flow
///
/// Always returns 200 OK regardless of whether the email exists, to prevent user enumeration.
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/forgot-password",
    tag = "admin-auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Request processed (same response whether email exists or not)"),
        (status = 500, description = "Internal server error")
    ),
    security() // No authentication required
)]
#[post("/auth/forgot-password")]
pub async fn forgot_password(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<ForgotPasswordRequest>,
) -> impl Responder {
    let mut user_uuid: Option<Uuid> = None;

    if let Some(svc) = data.password_reset_service() {
        match svc.request_reset(&body.email).await {
            Ok(Some(uuid)) => user_uuid = Some(uuid),
            Ok(None) => { /* user not found or throttled — silent */ }
            Err(e) => {
                log::error!("Password reset request failed: {e:?}");
                // Do not surface error to caller — always return generic 200
            }
        }
    } else {
        log::debug!("Password reset service not configured; ignoring forgot-password request");
    }

    if let Some(log_svc) = data.system_log_service() {
        log_svc
            .log_auth_event(
                None,
                user_uuid,
                "Password reset requested",
                Some(
                    serde_json::json!({"action": "password_reset_requested", "email": body.email}),
                ),
                SystemLogStatus::Success,
            )
            .await;
    }

    ApiResponse::message(
        "If an account with that email exists, a password reset link has been sent.",
    )
}

/// Reset password endpoint — validates the token and updates the password
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/reset-password",
    tag = "admin-auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful"),
        (status = 400, description = "Invalid or expired token, or validation error"),
        (status = 500, description = "Internal server error")
    ),
    security() // No authentication required
)]
#[post("/auth/reset-password")]
pub async fn reset_password(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<ResetPasswordRequest>,
) -> impl Responder {
    let Some(svc) = data.password_reset_service() else {
        log::warn!("Password reset service not configured");
        return ApiResponse::bad_request("Password reset is not available");
    };

    match svc.reset_password(&body.token, &body.new_password).await {
        Ok(user_uuid) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_auth_event(
                        Some(user_uuid),
                        Some(user_uuid),
                        "Password reset completed",
                        Some(serde_json::json!({"action": "password_reset_completed"})),
                        SystemLogStatus::Success,
                    )
                    .await;
            }
            ApiResponse::message("Password has been reset successfully.")
        }
        Err(r_data_core_core::error::Error::Validation(msg)) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_auth_event(
                        None,
                        None,
                        "Password reset failed",
                        Some(serde_json::json!({"action": "password_reset_failed", "reason": "validation"})),
                        SystemLogStatus::Failed,
                    )
                    .await;
            }
            ApiResponse::bad_request(&msg)
        }
        Err(e) => {
            log::error!("Password reset failed: {e:?}");
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_auth_event(
                        None,
                        None,
                        "Password reset failed",
                        Some(serde_json::json!({"action": "password_reset_failed", "reason": "internal_error"})),
                        SystemLogStatus::Failed,
                    )
                    .await;
            }
            ApiResponse::internal_error("Password reset failed")
        }
    }
}
