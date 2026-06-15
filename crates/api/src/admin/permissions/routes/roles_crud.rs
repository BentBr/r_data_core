#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::permissions::models::{CreateRoleRequest, RoleResponse, UpdateRoleRequest};
use crate::admin::query_helpers::to_list_query_params;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::query::StandardQuery;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{Permission, PermissionType, ResourceNamespace};
use r_data_core_services::query_validation::FieldValidator;

/// List all roles with pagination and sorting
#[utoipa::path(
    get,
    path = "/admin/api/v1/roles",
    tag = "roles",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100, or -1 for unlimited)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by (e.g., name, description, created_at)"),
        ("sort_order" = Option<String>, Query, description = "Sort order: 'asc' or 'desc' (default: 'asc')")
    ),
    responses(
        (status = 200, description = "List of roles with pagination", body = Vec<RoleResponse>),
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
pub async fn list_roles(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    query: web::Query<StandardQuery>,
) -> impl Responder {
    // Check permission
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Roles, &PermissionType::Read, None)
    {
        return resp;
    }

    // Create field validator
    let pool = Arc::new(state.db_pool().clone());
    let field_validator = Arc::new(FieldValidator::new(pool));

    // Convert StandardQuery to ListQueryParams and use service method that handles all validation
    let params = to_list_query_params(&query);
    let service = state.role_service();
    match service
        .list_roles_with_query(&params, &field_validator)
        .await
    {
        Ok((roles, validated)) => {
            // Get total count
            let total = service.count_roles().await.unwrap_or(0);

            let responses: Vec<RoleResponse> = roles.iter().map(RoleResponse::from).collect();

            ApiResponse::ok_paginated(responses, total, validated.page, validated.per_page)
        }
        Err(e) => {
            error!("Failed to list roles: {e}");
            match e {
                r_data_core_core::error::Error::Validation(msg) => {
                    ApiResponse::<()>::bad_request(&msg)
                }
                _ => ApiResponse::<()>::internal_error("Failed to retrieve roles"),
            }
        }
    }
}

/// Get a specific role by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/roles/{uuid}",
    tag = "roles",
    params(
        ("uuid" = Uuid, Path, description = "Role UUID")
    ),
    responses(
        (status = 200, description = "Role details", body = RoleResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Role not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
pub async fn get_role(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Roles, &PermissionType::Read, None)
    {
        return resp;
    }

    let uuid = path.into_inner();
    let service = state.role_service();

    match service.get_role(uuid).await {
        Ok(Some(role)) => ApiResponse::ok(RoleResponse::from(&role)),
        Ok(None) => ApiResponse::<()>::not_found("Role not found"),
        Err(e) => {
            error!("Failed to get role: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve role")
        }
    }
}

/// Create a new role
#[utoipa::path(
    post,
    path = "/admin/api/v1/roles",
    tag = "roles",
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created successfully", body = RoleResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 409, description = "Conflict - role name already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
pub async fn create_role(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    req: web::Json<CreateRoleRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Roles, &PermissionType::Create, None)
    {
        return resp;
    }

    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            let sub = &auth.0.sub;
            error!("Invalid UUID in auth token: {sub} - {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };

    let service = state.role_service();

    // Check if role with this name already exists
    if let Ok(Some(_)) = service.get_role_by_name(&req.name).await {
        return ApiResponse::<()>::conflict("A role with this name already exists");
    }

    // Convert request to domain model
    let mut permissions = Vec::new();
    for perm in &req.permissions {
        match Permission::try_from(perm.clone()) {
            Ok(p) => permissions.push(p),
            Err(e) => {
                error!("Invalid permission in request: {e}");
                return ApiResponse::<()>::bad_request(&format!("Invalid permission: {e}"));
            }
        }
    }

    // Validate: if super_admin is true, permissions should be empty (or ignored)
    let super_admin = req.super_admin.unwrap_or(false);
    if super_admin && !permissions.is_empty() {
        // Allow but warn - super_admin takes precedence
        log::warn!(
            "Role {} has super_admin=true but also has permissions - permissions will be ignored",
            req.name
        );
    }

    let mut role = r_data_core_core::permissions::role::Role::new(req.name.clone());
    role.description.clone_from(&req.description);
    role.super_admin = super_admin;
    role.permissions = permissions;

    match service.create_role(&role, creator_uuid).await {
        Ok(uuid) => {
            // Reload to get full details
            match service.get_role(uuid).await {
                Ok(Some(created_role)) => {
                    ApiResponse::<()>::created(RoleResponse::from(&created_role))
                }
                Ok(None) => ApiResponse::<()>::internal_error("Failed to retrieve created role"),
                Err(e) => {
                    error!("Failed to retrieve created role: {e}");
                    ApiResponse::<()>::internal_error("Failed to retrieve created role")
                }
            }
        }
        Err(e) => {
            error!("Failed to create role: {e}");
            ApiResponse::<()>::internal_error("Failed to create role")
        }
    }
}

/// Update an existing role
#[utoipa::path(
    put,
    path = "/admin/api/v1/roles/{uuid}",
    tag = "roles",
    params(
        ("uuid" = Uuid, Path, description = "Role UUID")
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated successfully", body = RoleResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Role not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}")]
pub async fn update_role(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<UpdateRoleRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Roles, &PermissionType::Update, None)
    {
        return resp;
    }

    let uuid = path.into_inner();
    let updater_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            let sub = &auth.0.sub;
            error!("Invalid UUID in auth token: {sub} - {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };

    let service = state.role_service();

    // Get existing role
    let mut role = match service.get_role(uuid).await {
        Ok(Some(r)) => r,
        Ok(None) => return ApiResponse::<()>::not_found("Role not found"),
        Err(e) => {
            error!("Failed to get role: {e}");
            return ApiResponse::<()>::internal_error("Failed to retrieve role");
        }
    };

    // Check if it's a system role
    if role.is_system {
        return ApiResponse::<()>::forbidden("Cannot modify system roles");
    }

    // Update fields
    role.name.clone_from(&req.name);
    role.description.clone_from(&req.description);
    if let Some(super_admin) = req.super_admin {
        role.super_admin = super_admin;
    }

    // Convert and update permissions
    let mut permissions = Vec::new();
    for perm in &req.permissions {
        match Permission::try_from(perm.clone()) {
            Ok(p) => permissions.push(p),
            Err(e) => {
                error!("Invalid permission in request: {e}");
                return ApiResponse::<()>::bad_request(&format!("Invalid permission: {e}"));
            }
        }
    }

    // Validate: if super_admin is true, permissions should be empty (or ignored)
    if role.super_admin && !permissions.is_empty() {
        // Allow but warn - super_admin takes precedence
        log::warn!(
            "Role {} has super_admin=true but also has permissions - permissions will be ignored",
            role.name
        );
    }

    role.permissions = permissions;

    match service.update_role(&role, updater_uuid).await {
        Ok(()) => {
            // Reload to get updated details
            match service.get_role(uuid).await {
                Ok(Some(updated_role)) => ApiResponse::ok(RoleResponse::from(&updated_role)),
                Ok(None) => ApiResponse::<()>::internal_error("Failed to retrieve updated role"),
                Err(e) => {
                    error!("Failed to retrieve updated role: {e}");
                    ApiResponse::<()>::internal_error("Failed to retrieve updated role")
                }
            }
        }
        Err(e) => {
            error!("Failed to update role: {e}");
            ApiResponse::<()>::internal_error("Failed to update role")
        }
    }
}

/// Delete a role
#[utoipa::path(
    delete,
    path = "/admin/api/v1/roles/{uuid}",
    tag = "roles",
    params(
        ("uuid" = Uuid, Path, description = "Role UUID")
    ),
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Role not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
pub async fn delete_role(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Roles, &PermissionType::Delete, None)
    {
        return resp;
    }

    let uuid = path.into_inner();
    let actor_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(u) => u,
        Err(e) => {
            let sub = &auth.0.sub;
            error!("Invalid UUID in auth token: {sub} - {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };
    let service = state.role_service();

    // Check if it's a system role
    if let Ok(Some(role)) = service.get_role(uuid).await {
        if role.is_system {
            return ApiResponse::<()>::forbidden("Cannot delete system roles");
        }
    }

    match service.delete_role(uuid, actor_uuid).await {
        Ok(()) => ApiResponse::ok_with_message((), "Role deleted successfully"),
        Err(e) => {
            error!("Failed to delete role: {e}");
            ApiResponse::<()>::internal_error("Failed to delete role")
        }
    }
}
