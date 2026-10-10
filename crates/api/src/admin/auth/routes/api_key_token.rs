#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

//! Trading an API key for a short-lived admin access token.
//!
//! The admin API requires a JWT — that is the documented model, and widening
//! every admin route to accept API keys would change the product's security
//! posture wholesale. But a machine caller cannot hold a JWT: they last thirty
//! minutes. So it presents its key here, once, and receives a token minted for
//! the key's owner with that owner's roles.
//!
//! This is the same shape as the OIDC exchange next door, and for the same
//! reason: one explicit, auditable door, rather than a second authentication
//! path spread across forty handlers.
//!
//! The token is bounded by the owner's roles and expires like any other, so a
//! leaked key is no more powerful than the account behind it and stops working
//! the moment the key is revoked — the exchange is where that is checked.

use actix_web::{post, web, Responder};
use std::sync::Arc;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use crate::response::ApiResponse;
use r_data_core_core::admin_jwt::{generate_access_token, ACCESS_TOKEN_EXPIRY_SECONDS};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};

/// Exchange an API key for an admin access token.
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/api-key/token",
    tag = "admin-auth",
    responses(
        (status = 200, description = "A short-lived access token for the key's owner"),
        (status = 401, description = "No usable API key was presented"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    )
)]
#[post("/auth/api-key/token")]
pub async fn exchange_api_key(
    data: web::Data<ApiStateWrapper>,
    auth: CombinedRequiredAuth,
) -> impl Responder {
    // Deliberately not accepting a JWT here: someone holding one already has
    // what this endpoint issues, and allowing it would turn the endpoint into
    // a way to mint a fresh token from an expiring one, indefinitely.
    let Some(key) = auth.api_key_info.as_ref() else {
        return ApiResponse::<()>::unauthorized("An API key is required");
    };

    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));
    let Ok(Some(user)) = repo.find_by_uuid(&key.user_uuid).await else {
        log::error!("An API key resolved to no account; refusing to issue a token");
        return ApiResponse::<()>::unauthorized("An API key is required");
    };

    // The key's owner decides what the token can do. A key is never more
    // powerful than the account behind it.
    if !user.is_active {
        return ApiResponse::<()>::forbidden("This account is not permitted to sign in");
    }

    let roles = super::helpers::load_user_roles(&user, &data, &repo).await;
    let Ok(access_token) = generate_access_token(&user, data.api_config(), &roles) else {
        log::error!("Failed to mint an access token for an API key");
        return ApiResponse::<()>::internal_error("Could not issue a token");
    };

    // No refresh token. The caller re-exchanges with the key it already holds,
    // which keeps revocation immediate: revoke the key and the next exchange
    // fails, rather than a refresh token outliving it for thirty days.
    ApiResponse::ok(serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_EXPIRY_SECONDS,
    }))
}
