#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::users::models::{CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::query::PaginationQuery;
use crate::response::ApiResponse;
use r_data_core_core::admin_user::UserRole;
use r_data_core_core::permissions::permission_scheme::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
};
use std::str::FromStr;
use validator::Validate;

/// Register user routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_users)
        .service(get_user)
        .service(create_user)
        .service(update_user)
        .service(delete_user)
        .service(get_user_schemes)
        .service(assign_schemes_to_user);
}

/// List all users with pagination
#[utoipa::path(
    get,
    path = "/admin/api/v1/users",
    tag = "users",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)")
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
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    // Check permission - need PermissionSchemes:Admin to manage users
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    match repo.list_admin_users(limit, offset).await {
        Ok(users) => {
            // For now, we don't have a count method, so we'll use the length
            // In a real implementation, you'd want a separate count query
            #[allow(clippy::cast_possible_wrap)]
            let total = users.len() as i64;
            let responses: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
            ApiResponse::ok_paginated(responses, total, page, per_page)
        }
        Err(e) => {
            error!("Failed to list users: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve users")
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
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    match repo.find_by_uuid(&user_uuid).await {
        Ok(Some(user)) => ApiResponse::ok(UserResponse::from(&user)),
        Ok(None) => ApiResponse::<()>::not_found("User not found"),
        Err(e) => {
            error!("Failed to get user: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve user")
        }
    }
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/admin/api/v1/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
pub async fn create_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    req: web::Json<CreateUserRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    // Validate request
    if let Err(errors) = req.validate() {
        return ApiResponse::unprocessable_entity(&format!("Validation error: {errors}"));
    }

    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Check if username or email already exists
    if let Ok(Some(_)) = repo.find_by_username_or_email(&req.username).await {
        return ApiResponse::conflict("Username already exists");
    }

    if let Ok(Some(_)) = repo.find_by_username_or_email(&req.email).await {
        return ApiResponse::conflict("Email already in use");
    }

    // Extract creator UUID from auth claims
    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Failed to parse creator UUID from claims: {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication");
        }
    };

    // Create user
    let params = CreateAdminUserParams {
        username: &req.username,
        email: &req.email,
        password: &req.password,
        first_name: &req.first_name,
        last_name: &req.last_name,
        role: req.role.as_deref(),
        is_active: req.is_active.unwrap_or(true),
        creator_uuid,
    };

    match repo.create_admin_user(&params).await {
        Ok(user_uuid) => {
            // Fetch the created user to return
            match repo.find_by_uuid(&user_uuid).await {
                Ok(Some(mut user)) => {
                    // Update super_admin flag if provided
                    if req.super_admin.unwrap_or(false) {
                        user.super_admin = true;
                        if let Err(e) = repo.update_admin_user(&user).await {
                            error!("Failed to update super_admin flag: {e}");
                            return ApiResponse::<()>::internal_error(
                                "User created but failed to set super_admin flag",
                            );
                        }
                        // Re-fetch to get updated user
                        if let Ok(Some(updated)) = repo.find_by_uuid(&user_uuid).await {
                            user = updated;
                        }
                    }
                    // Invalidate cache for the new user
                    state
                        .permission_scheme_service()
                        .invalidate_user_permissions_cache(&user_uuid)
                        .await;
                    ApiResponse::<UserResponse>::created(UserResponse::from(&user))
                }
                Ok(None) => ApiResponse::<()>::internal_error("User created but not found"),
                Err(e) => {
                    error!("Failed to fetch created user: {e}");
                    ApiResponse::<()>::internal_error("User created but failed to retrieve")
                }
            }
        }
        Err(e) => {
            error!("Failed to create user: {e}");
            ApiResponse::<()>::internal_error("Failed to create user")
        }
    }
}

/// Update a user
#[utoipa::path(
    put,
    path = "/admin/api/v1/users/{uuid}",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserResponse),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}")]
pub async fn update_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<UpdateUserRequest>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    // Validate request
    if let Err(errors) = req.validate() {
        return ApiResponse::unprocessable_entity(&format!("Validation error: {errors}"));
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Get existing user
    let mut user = match repo.find_by_uuid(&user_uuid).await {
        Ok(Some(u)) => u,
        Ok(None) => return ApiResponse::<()>::not_found("User not found"),
        Err(e) => {
            error!("Failed to get user: {e}");
            return ApiResponse::<()>::internal_error("Failed to retrieve user");
        }
    };

    // Update fields if provided
    if let Some(email) = &req.email {
        // Check if email is already in use by another user
        if let Ok(Some(existing)) = repo.find_by_username_or_email(email).await {
            if existing.uuid != user_uuid {
                return ApiResponse::conflict("Email already in use");
            }
        }
        user.email.clone_from(email);
    }

    if let Some(first_name) = &req.first_name {
        user.first_name = Some(first_name.clone());
    }

    if let Some(last_name) = &req.last_name {
        user.last_name = Some(last_name.clone());
    }

    // Update full_name if first_name or last_name changed
    if req.first_name.is_some() || req.last_name.is_some() {
        user.full_name = format!(
            "{} {}",
            user.first_name.as_deref().unwrap_or(""),
            user.last_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string();
        if user.full_name.is_empty() {
            user.full_name.clone_from(&user.username);
        }
    }

    if let Some(role_str) = &req.role {
        user.role =
            UserRole::from_str(role_str).unwrap_or_else(|()| UserRole::Custom(role_str.clone()));
    }

    if let Some(is_active) = req.is_active {
        user.is_active = is_active;
    }

    if let Some(super_admin) = req.super_admin {
        user.super_admin = super_admin;
    }

    // Update password if provided
    if let Some(password) = &req.password {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        use argon2::Argon2;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(hash) => {
                user.password_hash = hash.to_string();
            }
            Err(e) => {
                error!("Failed to hash password: {e}");
                return ApiResponse::<()>::internal_error("Failed to update password");
            }
        }
    }

    // Update user
    match repo.update_admin_user(&user).await {
        Ok(()) => {
            // Invalidate cache for the updated user
            state
                .permission_scheme_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            ApiResponse::ok(UserResponse::from(&user))
        }
        Err(e) => {
            error!("Failed to update user: {e}");
            ApiResponse::<()>::internal_error("Failed to update user")
        }
    }
}

/// Delete a user
#[utoipa::path(
    delete,
    path = "/admin/api/v1/users/{uuid}",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
pub async fn delete_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    match repo.delete_admin_user(&user_uuid).await {
        Ok(()) => {
            // Invalidate cache for the deleted user
            state
                .permission_scheme_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            ApiResponse::ok_with_message((), "User deleted successfully")
        }
        Err(e) => {
            error!("Failed to delete user: {e}");
            ApiResponse::<()>::internal_error("Failed to delete user")
        }
    }
}

/// Get user's permission schemes
#[utoipa::path(
    get,
    path = "/admin/api/v1/users/{uuid}/schemes",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "List of permission scheme UUIDs", body = Vec<Uuid>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}/schemes")]
pub async fn get_user_schemes(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    match repo.get_user_permission_schemes(user_uuid).await {
        Ok(scheme_uuids) => ApiResponse::ok(scheme_uuids),
        Err(e) => {
            error!("Failed to get user permission schemes: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve permission schemes")
        }
    }
}

/// Assign permission schemes to a user
#[utoipa::path(
    put,
    path = "/admin/api/v1/users/{uuid}/schemes",
    tag = "users",
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body(content = Vec<Uuid>, description = "List of permission scheme UUIDs"),
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
#[put("/{uuid}/schemes")]
pub async fn assign_schemes_to_user(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    path: web::Path<Uuid>,
    req: web::Json<Vec<Uuid>>,
) -> impl Responder {
    // Check permission
    if let Err(resp) = auth.require_permission(
        &ResourceNamespace::PermissionSchemes,
        &PermissionType::Admin,
        None,
    ) {
        return resp;
    }

    let user_uuid = path.into_inner();
    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

    // Verify user exists
    if matches!(repo.find_by_uuid(&user_uuid).await, Ok(None)) {
        return ApiResponse::<()>::not_found("User not found");
    }

    match repo.update_user_schemes(user_uuid, &req.into_inner()).await {
        Ok(()) => {
            // Invalidate cached permissions for this user
            state
                .permission_scheme_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            ApiResponse::ok_with_message((), "Permission schemes assigned successfully")
        }
        Err(e) => {
            error!("Failed to assign permission schemes to user: {e}");
            ApiResponse::<()>::internal_error("Failed to assign permission schemes")
        }
    }
}
