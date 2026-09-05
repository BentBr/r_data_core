#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

use actix_web::{post, web, Responder};
use uuid::Uuid;

use crate::admin::auth::models::{LogoutRequest, RefreshTokenRequest, RefreshTokenResponse};
use crate::admin::auth::routes::helpers::{build_refresh_response, load_user_roles};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::response::ApiResponse;
use crate::token_service::TokenService;
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_core::system_log::SystemLogStatus;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};

/// Logout endpoint for admin users
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/logout",
    tag = "admin-auth",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful"),
        (status = 400, description = "Invalid request format"),
        (status = 401, description = "Invalid refresh token"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/logout")]
pub async fn admin_logout(
    data: web::Data<ApiStateWrapper>,
    request: web::Json<LogoutRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());

    // Hash the provided refresh token
    let token_hash = match RefreshToken::hash_token(&request.refresh_token) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash refresh token for logout: {e:?}");
            return ApiResponse::bad_request("Invalid token format");
        }
    };

    // Revoke the refresh token
    match refresh_repo.revoke_by_token_hash(&token_hash).await {
        Ok(()) => {
            log::info!("User logged out successfully, refresh token revoked");
            ApiResponse::message("Logout successful")
        }
        Err(e) => {
            log::error!("Failed to revoke refresh token during logout: {e:?}");
            ApiResponse::internal_error("Logout failed")
        }
    }
}

/// Refresh access token endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/refresh",
    tag = "admin-auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshTokenResponse),
        (status = 400, description = "Invalid request format"),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/refresh")]
pub async fn admin_refresh_token(
    data: web::Data<ApiStateWrapper>,
    request: web::Json<RefreshTokenRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());
    let admin_repo = AdminUserRepository::new(std::sync::Arc::new(data.db_pool().clone()));

    // Hash the provided refresh token
    let token_hash = match RefreshToken::hash_token(&request.refresh_token) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash refresh token: {e:?}");
            return ApiResponse::unauthorized("Invalid refresh token");
        }
    };

    // Find the refresh token in database
    let refresh_token = match refresh_repo.find_by_token_hash(&token_hash).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            log::warn!("Refresh token not found");
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_auth_event(
                        None,
                        None,
                        "Token refresh failed: token not found",
                        Some(serde_json::json!({"action": "refresh", "reason": "token_not_found"})),
                        SystemLogStatus::Failed,
                    )
                    .await;
            }
            return ApiResponse::unauthorized("Invalid refresh token");
        }
        Err(e) => {
            log::error!("Database error finding refresh token: {e:?}");
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Check if token is valid
    if !refresh_token.is_valid() {
        log::warn!("Refresh token is expired or revoked");
        if let Some(log_svc) = data.system_log_service() {
            log_svc
                .log_auth_event(
                    None,
                    Some(refresh_token.user_id),
                    "Token refresh failed: expired or revoked",
                    Some(serde_json::json!({"action": "refresh", "reason": "expired_or_revoked"})),
                    SystemLogStatus::Failed,
                )
                .await;
        }
        return ApiResponse::unauthorized("Refresh token expired or revoked");
    }

    // Get the user
    let user = match admin_repo.find_by_uuid(&refresh_token.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            log::error!("User not found for refresh token");
            return ApiResponse::unauthorized("Invalid refresh token");
        }
        Err(e) => {
            log::error!("Database error finding user: {e:?}");
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Check if user is still active
    if !user.is_active {
        log::warn!(
            "Attempt to refresh token for inactive user: {}",
            user.username
        );
        return ApiResponse::unauthorized("Account not active");
    }

    // Load roles and generate tokens via TokenService
    let roles = load_user_roles(&user, &data, &admin_repo).await;
    let token_service = TokenService::new(data.api_config());
    let token_pair = match token_service.generate_token_pair(&user, &roles) {
        Ok(pair) => pair,
        Err(e) => {
            log::error!("Failed to generate tokens: {e:?}");
            return ApiResponse::internal_error("Token refresh failed");
        }
    };

    // Update the old refresh token as used
    if let Err(e) = refresh_repo.update_last_used(refresh_token.id).await {
        log::error!("Failed to update refresh token last used: {e:?}");
    }

    // Create new refresh token in database
    let device_info = refresh_token.device_info.clone();
    if let Err(e) = refresh_repo
        .create(
            user.uuid,
            token_pair.refresh_token_hash,
            token_pair.refresh_expires_at,
            device_info,
        )
        .await
    {
        log::error!("Failed to store new refresh token: {e:?}");
        return ApiResponse::internal_error("Token refresh failed");
    }

    // Revoke the old refresh token
    if let Err(e) = refresh_repo.revoke_by_id(refresh_token.id).await {
        log::error!("Failed to revoke old refresh token: {e:?}");
        // Continue anyway since new token was created
    }

    // Log successful token refresh
    if let Some(log_svc) = data.system_log_service() {
        log_svc
            .log_auth_event(
                Some(user.uuid),
                Some(user.uuid),
                "Token refreshed",
                Some(serde_json::json!({"action": "refresh"})),
                SystemLogStatus::Success,
            )
            .await;
    }

    // Build response
    build_refresh_response(
        token_pair.access_token,
        token_pair.refresh_token,
        token_pair.access_expires_at,
        token_pair.refresh_expires_at,
    )
}

/// Revoke all refresh tokens for current user endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/revoke-all",
    tag = "admin-auth",
    responses(
        (status = 200, description = "All tokens revoked successfully"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/auth/revoke-all")]
pub async fn admin_revoke_all_tokens(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Extract user claims from JWT (already extracted via RequiredAuth extractor)
    let claims = auth.0;

    let Ok(user_uuid) = Uuid::parse_str(&claims.sub) else {
        return ApiResponse::unauthorized("Invalid user ID in token");
    };

    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());

    // Revoke all refresh tokens for the user
    match refresh_repo.revoke_all_for_user(user_uuid).await {
        Ok(count) => {
            let name = &claims.name;
            log::info!("Revoked {count} refresh tokens for user {name}");
            ApiResponse::ok(format!("Revoked {count} active sessions"))
        }
        Err(e) => {
            log::error!(
                "Failed to revoke all tokens for user {}: {:?}",
                claims.name,
                e
            );
            ApiResponse::internal_error("Failed to revoke tokens")
        }
    }
}
