#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};

/// Get user's roles
#[utoipa::path(
    get,
    path = "/admin/api/v1/users/{uuid}/roles",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "List of role UUIDs", body = Vec<Uuid>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}/roles")]
pub async fn get_user_roles(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission - need Users:Read to get user roles
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Read, None)
    {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    match repo.get_user_roles(user_uuid).await {
        Ok(role_uuids) => ApiResponse::ok(role_uuids),
        Err(e) => {
            error!("Failed to get user roles: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve roles")
        }
    }
}

/// Assign roles to a user
#[utoipa::path(
    put,
    path = "/admin/api/v1/users/{uuid}/roles",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body(content = Vec<Uuid>, description = "List of role UUIDs"),
    responses(
        (status = 200, description = "Roles assigned successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}/roles")]
pub async fn assign_roles_to_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<Vec<Uuid>>,
) -> impl Responder {
    // Check permission - need Users:Update to assign roles to users
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Update, None)
    {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    match repo.update_user_roles(user_uuid, &req.into_inner()).await {
        Ok(()) => {
            // Invalidate cached permissions for this user
            state
                .role_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            ApiResponse::ok_with_message((), "Roles assigned successfully")
        }
        Err(e) => {
            error!("Failed to assign roles to user: {e}");
            ApiResponse::<()>::internal_error("Failed to assign roles")
        }
    }
}
