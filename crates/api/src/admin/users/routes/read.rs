#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Reading users.
//!
//! Split from the write handlers because they answer a different question and
//! carry different risk: nothing here changes anything, so the guards worth
//! reading all live next door.

use actix_web::{get, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::query_helpers::to_list_query_params;
use crate::admin::users::models::UserResponse;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::query::StandardQuery;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_services::query_validation::FieldValidator;

/// List all users with pagination and sorting
#[utoipa::path(
    get,
    path = "/admin/api/v1/users",
    tag = "users",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100, or -1 for unlimited)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by (e.g., username, email, created_at)"),
        ("sort_order" = Option<String>, Query, description = "Sort order: 'asc' or 'desc' (default: 'asc')")
    ),
    responses(
        (status = 200, description = "List of users with pagination", body = Vec<UserResponse>),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("")]
pub async fn list_users(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    query: web::Query<StandardQuery>,
) -> impl Responder {
    // Check permission - need Users:Read to list users
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Read, None)
    {
        return resp;
    }

    // Create field validator
    let pool = Arc::new(state.db_pool().clone());
    let field_validator = Arc::new(FieldValidator::new(pool));

    // Convert StandardQuery to ListQueryParams and use service method that handles all validation
    let params = to_list_query_params(&query);
    let service = state.admin_user_service();
    match service
        .list_users_with_query(&params, &field_validator)
        .await
    {
        Ok((users, validated)) => {
            // For now, we don't have a count method, so we'll use the length
            // In a real implementation, you'd want a separate count query
            let total = i64::try_from(users.len()).unwrap_or(0);
            // Load role_uuids for each user
            let pool = Arc::new(state.db_pool().clone());
            let repo = AdminUserRepository::new(pool);
            let mut responses = Vec::new();
            for user in &users {
                let role_uuids = repo.get_user_roles(user.uuid).await.unwrap_or_default();
                responses.push(UserResponse::from_with_roles(user, &role_uuids));
            }
            ApiResponse::ok_paginated(responses, total, validated.page, validated.per_page)
        }
        Err(e) => {
            error!("Failed to list users: {e}");
            match e {
                r_data_core_core::error::Error::Validation(msg) => {
                    ApiResponse::<()>::bad_request(&msg)
                }
                _ => ApiResponse::<()>::internal_error("Failed to retrieve users"),
            }
        }
    }
}

/// Get a user by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/users/{uuid}",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User details", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
pub async fn get_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission - need Users:Read to get user
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Read, None)
    {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    match repo.find_by_uuid(&user_uuid).await {
        Ok(Some(user)) => {
            let role_uuids = repo.get_user_roles(user_uuid).await.unwrap_or_default();
            ApiResponse::ok(UserResponse::from_with_roles(&user, &role_uuids))
        }
        Ok(None) => ApiResponse::<()>::not_found("User not found"),
        Err(e) => {
            error!("Failed to get user: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve user")
        }
    }
}
