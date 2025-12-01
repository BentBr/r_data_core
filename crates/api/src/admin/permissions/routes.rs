#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::permissions::models::{
    AssignSchemesRequest, CreatePermissionSchemeRequest, PermissionSchemeResponse,
    UpdatePermissionSchemeRequest,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::auth::RequiredAuthExt;
use crate::query::PaginationQuery;
use crate::response::ApiResponse;
use r_data_core_core::permissions::permission_scheme::{
    Permission, PermissionType, ResourceNamespace,
};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, ApiKeyRepositoryTrait,
};

/// Register permission routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_permission_schemes)
        .service(get_permission_scheme)
        .service(create_permission_scheme)
        .service(update_permission_scheme)
        .service(delete_permission_scheme)
        .service(assign_schemes_to_user)
        .service(assign_schemes_to_api_key);
}

/// List all permission schemes with pagination
#[utoipa::path(
    get,
    path = "/admin/api/v1/permissions",
    tag = "permissions",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)")
    ),
    responses(
        (status = 200, description = "List of permission schemes with pagination", body = Vec<PermissionSchemeResponse>),
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
pub async fn list_permission_schemes(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Read,
        None,
    ) {
        return resp;
    }

    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Read,
        None,
    ) {
        return resp;
    }

    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    let service = state.permission_scheme_service();

    // Get both the schemes and the total count
    let (schemes_result, count_result) =
        tokio::join!(service.list_schemes(limit, offset), service.count_schemes());

    match (schemes_result, count_result) {
        (Ok(schemes), Ok(total)) => {
            let responses: Vec<PermissionSchemeResponse> =
                schemes.iter().map(PermissionSchemeResponse::from).collect();

            ApiResponse::ok_paginated(responses, total, page, per_page)
        }
        (Err(e), _) => {
            error!("Failed to list permission schemes: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve permission schemes")
        }
        (_, Err(e)) => {
            error!("Failed to count permission schemes: {}", e);
            ApiResponse::<()>::internal_error("Failed to count permission schemes")
        }
    }
}

/// Get a specific permission scheme by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/permissions/{uuid}",
    tag = "permissions",
    params(
        ("uuid" = Uuid, Path, description = "Permission scheme UUID")
    ),
    responses(
        (status = 200, description = "Permission scheme details", body = PermissionSchemeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Permission scheme not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
pub async fn get_permission_scheme(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Read,
        None,
    ) {
        return resp;
    }

    let uuid = path.into_inner();
    let service = state.permission_scheme_service();

    match service.get_scheme(uuid).await {
        Ok(Some(scheme)) => ApiResponse::ok(PermissionSchemeResponse::from(&scheme)),
        Ok(None) => ApiResponse::<()>::not_found("Permission scheme not found"),
        Err(e) => {
            error!("Failed to get permission scheme: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve permission scheme")
        }
    }
}

/// Create a new permission scheme
#[utoipa::path(
    post,
    path = "/admin/api/v1/permissions",
    tag = "permissions",
    request_body = CreatePermissionSchemeRequest,
    responses(
        (status = 201, description = "Permission scheme created successfully", body = PermissionSchemeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 409, description = "Conflict - permission scheme name already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
pub async fn create_permission_scheme(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    req: web::Json<CreatePermissionSchemeRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Create,
        None,
    ) {
        return resp;
    }

    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid UUID in auth token: {} - {}", auth.0.sub, e);
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };

    let service = state.permission_scheme_service();

    // Check if scheme with this name already exists
    if let Ok(Some(_)) = service.get_scheme_by_name(&req.name).await {
        return ApiResponse::<()>::conflict("A permission scheme with this name already exists");
    }

    // Convert request to domain model
    let mut role_permissions = std::collections::HashMap::new();
    for (role, permissions) in &req.role_permissions {
        let mut converted_permissions = Vec::new();
        for perm in permissions {
            match Permission::try_from(perm.clone()) {
                Ok(p) => converted_permissions.push(p),
                Err(e) => {
                    error!("Invalid permission in request: {}", e);
                    return ApiResponse::<()>::bad_request(&format!("Invalid permission: {}", e));
                }
            }
        }
        role_permissions.insert(role.clone(), converted_permissions);
    }

    let mut scheme =
        r_data_core_core::permissions::permission_scheme::PermissionScheme::new(req.name.clone());
    scheme.description = req.description.clone();
    scheme.role_permissions = role_permissions;

    match service.create_scheme(&scheme, creator_uuid).await {
        Ok(uuid) => {
            // Reload to get full details
            match service.get_scheme(uuid).await {
                Ok(Some(created_scheme)) => {
                    ApiResponse::<()>::created(PermissionSchemeResponse::from(&created_scheme))
                }
                Ok(None) => ApiResponse::<()>::internal_error("Failed to retrieve created scheme"),
                Err(e) => {
                    error!("Failed to retrieve created scheme: {}", e);
                    ApiResponse::<()>::internal_error("Failed to retrieve created scheme")
                }
            }
        }
        Err(e) => {
            error!("Failed to create permission scheme: {}", e);
            ApiResponse::<()>::internal_error("Failed to create permission scheme")
        }
    }
}

/// Update an existing permission scheme
#[utoipa::path(
    put,
    path = "/admin/api/v1/permissions/{uuid}",
    tag = "permissions",
    params(
        ("uuid" = Uuid, Path, description = "Permission scheme UUID")
    ),
    request_body = UpdatePermissionSchemeRequest,
    responses(
        (status = 200, description = "Permission scheme updated successfully", body = PermissionSchemeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Permission scheme not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}")]
pub async fn update_permission_scheme(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<UpdatePermissionSchemeRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Update,
        None,
    ) {
        return resp;
    }

    let uuid = path.into_inner();
    let updater_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Invalid UUID in auth token: {} - {}", auth.0.sub, e);
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };

    let service = state.permission_scheme_service();

    // Get existing scheme
    let mut scheme = match service.get_scheme(uuid).await {
        Ok(Some(s)) => s,
        Ok(None) => return ApiResponse::<()>::not_found("Permission scheme not found"),
        Err(e) => {
            error!("Failed to get permission scheme: {}", e);
            return ApiResponse::<()>::internal_error("Failed to retrieve permission scheme");
        }
    };

    // Check if it's a system scheme
    if scheme.is_system {
        return ApiResponse::<()>::forbidden("Cannot modify system permission schemes");
    }

    // Update fields
    scheme.name = req.name.clone();
    scheme.description = req.description.clone();

    // Convert and update permissions
    let mut role_permissions = std::collections::HashMap::new();
    for (role, permissions) in &req.role_permissions {
        let mut converted_permissions = Vec::new();
        for perm in permissions {
            match Permission::try_from(perm.clone()) {
                Ok(p) => converted_permissions.push(p),
                Err(e) => {
                    error!("Invalid permission in request: {}", e);
                    return ApiResponse::<()>::bad_request(&format!("Invalid permission: {}", e));
                }
            }
        }
        role_permissions.insert(role.clone(), converted_permissions);
    }
    scheme.role_permissions = role_permissions;

    match service.update_scheme(&scheme, updater_uuid).await {
        Ok(()) => {
            // Reload to get updated details
            match service.get_scheme(uuid).await {
                Ok(Some(updated_scheme)) => {
                    ApiResponse::ok(PermissionSchemeResponse::from(&updated_scheme))
                }
                Ok(None) => ApiResponse::<()>::internal_error("Failed to retrieve updated scheme"),
                Err(e) => {
                    error!("Failed to retrieve updated scheme: {}", e);
                    ApiResponse::<()>::internal_error("Failed to retrieve updated scheme")
                }
            }
        }
        Err(e) => {
            error!("Failed to update permission scheme: {}", e);
            ApiResponse::<()>::internal_error("Failed to update permission scheme")
        }
    }
}

/// Delete a permission scheme
#[utoipa::path(
    delete,
    path = "/admin/api/v1/permissions/{uuid}",
    tag = "permissions",
    params(
        ("uuid" = Uuid, Path, description = "Permission scheme UUID")
    ),
    responses(
        (status = 200, description = "Permission scheme deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "Permission scheme not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
pub async fn delete_permission_scheme(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Delete,
        None,
    ) {
        return resp;
    }

    let uuid = path.into_inner();
    let service = state.permission_scheme_service();

    // Check if it's a system scheme
    if let Ok(Some(scheme)) = service.get_scheme(uuid).await {
        if scheme.is_system {
            return ApiResponse::<()>::forbidden("Cannot delete system permission schemes");
        }
    }

    match service.delete_scheme(uuid).await {
        Ok(()) => ApiResponse::ok_with_message((), "Permission scheme deleted successfully"),
        Err(e) => {
            error!("Failed to delete permission scheme: {}", e);
            ApiResponse::<()>::internal_error("Failed to delete permission scheme")
        }
    }
}

/// Assign permission schemes to a user
#[utoipa::path(
    put,
    path = "/admin/api/v1/permissions/users/{user_uuid}/schemes",
    tag = "permissions",
    params(
        ("user_uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body = AssignSchemesRequest,
    responses(
        (status = 200, description = "Permission schemes assigned successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/users/{user_uuid}/schemes")]
pub async fn assign_schemes_to_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<AssignSchemesRequest>,
) -> impl Responder {
    // Check permission (need permission to manage users or permission schemes)
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden(
            "Insufficient permissions to assign permission schemes",
        );
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if let Ok(None) = repo.find_by_uuid(&user_uuid).await {
        return ApiResponse::<()>::not_found("User not found");
    }

    // Update user's permission schemes
    match repo.update_user_schemes(user_uuid, &req.scheme_uuids).await {
        Ok(()) => {
            // Invalidate cached permissions for this user
            state
                .permission_scheme_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            ApiResponse::ok_with_message((), "Permission schemes assigned successfully")
        }
        Err(e) => {
            error!("Failed to assign permission schemes to user: {}", e);
            ApiResponse::<()>::internal_error("Failed to assign permission schemes")
        }
    }
}

/// Assign permission schemes to an API key
#[utoipa::path(
    put,
    path = "/admin/api/v1/permissions/api-keys/{api_key_uuid}/schemes",
    tag = "permissions",
    params(
        ("api_key_uuid" = Uuid, Path, description = "API key UUID")
    ),
    request_body = AssignSchemesRequest,
    responses(
        (status = 200, description = "Permission schemes assigned successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/api-keys/{api_key_uuid}/schemes")]
pub async fn assign_schemes_to_api_key(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<AssignSchemesRequest>,
) -> impl Responder {
    // Check permission (need permission to manage API keys or permission schemes)
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden(
            "Insufficient permissions to assign permission schemes",
        );
    }

    let api_key_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = ApiKeyRepository::new(pool);

    // Verify API key exists
    if let Ok(None) = repo.get_by_uuid(api_key_uuid).await {
        return ApiResponse::<()>::not_found("API key not found");
    }

    // Update API key's permission schemes
    match repo
        .update_api_key_schemes(api_key_uuid, &req.scheme_uuids)
        .await
    {
        Ok(()) => {
            // Invalidate cached permissions for this API key
            state
                .permission_scheme_service()
                .invalidate_api_key_permissions_cache(&api_key_uuid)
                .await;
            ApiResponse::ok_with_message((), "Permission schemes assigned successfully")
        }
        Err(e) => {
            error!("Failed to assign permission schemes to API key: {}", e);
            ApiResponse::<()>::internal_error("Failed to assign permission schemes")
        }
    }
}
