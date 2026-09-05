#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::permissions::models::AssignRolesRequest;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, ApiKeyRepositoryTrait,
};

/// Assign roles to a user
#[utoipa::path(
    put,
    path = "/admin/api/v1/roles/users/{user_uuid}/roles",
    tag = "roles",
    params(
        ("user_uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body = AssignRolesRequest,
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
#[put("/users/{user_uuid}/roles")]
pub async fn assign_roles_to_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<AssignRolesRequest>,
) -> impl Responder {
    // Check permission (need permission to manage users or roles)
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Roles,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to assign roles");
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    // Update user's roles
    match repo.update_user_roles(user_uuid, &req.role_uuids).await {
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

/// Assign roles to an API key
#[utoipa::path(
    put,
    path = "/admin/api/v1/roles/api-keys/{api_key_uuid}/roles",
    tag = "roles",
    params(
        ("api_key_uuid" = Uuid, Path, description = "API key UUID")
    ),
    request_body = AssignRolesRequest,
    responses(
        (status = 200, description = "Roles assigned successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/api-keys/{api_key_uuid}/roles")]
pub async fn assign_roles_to_api_key(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<AssignRolesRequest>,
) -> impl Responder {
    // Check permission (need permission to manage API keys or roles)
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Roles,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to assign roles");
    }

    let api_key_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = ApiKeyRepository::new(pool);

    // Verify API key exists
    if matches!(repo.get_by_uuid(api_key_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("API key not found");
    }

    // Update API key's roles
    match repo
        .update_api_key_roles(api_key_uuid, &req.role_uuids)
        .await
    {
        Ok(()) => {
            // Invalidate cached permissions for this API key
            state
                .role_service()
                .invalidate_api_key_permissions_cache(&api_key_uuid)
                .await;
            ApiResponse::ok_with_message((), "Roles assigned successfully")
        }
        Err(e) => {
            error!("Failed to assign roles to API key: {e}");
            ApiResponse::<()>::internal_error("Failed to assign roles")
        }
    }
}
