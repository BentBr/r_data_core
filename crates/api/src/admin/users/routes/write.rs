#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Creating, changing and removing users.
//!
//! Split from the read handlers so the guards that matter — who may change
//! what, and the refusal to set a password on a federated account — sit
//! together rather than buried among listings.

use actix_web::{delete, post, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::users::models::{CreateUserRequest, UpdateUserRequest, UserResponse};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::RequiredAuthExt;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use validator::Validate;

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
    // Check permission - need Users:Create to create users
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Create, None)
    {
        return resp;
    }

    // Validate request
    if let Err(errors) = req.validate() {
        return ApiResponse::unprocessable_entity(&format!("Validation error: {errors}"));
    }

    // Extract creator UUID from auth claims
    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Failed to parse creator UUID from claims: {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication");
        }
    };

    // Create user via service (handles duplicate checks and audit logging)
    let service = state.admin_user_service();
    let user_uuid = match service
        .register_user(
            &req.username,
            &req.email,
            &req.password,
            &req.first_name,
            &req.last_name,
            None, // No longer using role field
            req.is_active.unwrap_or(true),
            creator_uuid,
        )
        .await
    {
        Ok(uuid) => uuid,
        Err(r_data_core_core::error::Error::Validation(msg))
            if msg.contains("already exists") || msg.contains("already in use") =>
        {
            return ApiResponse::conflict(&msg);
        }
        Err(e) => {
            error!("Failed to create user: {e}");
            return ApiResponse::<()>::internal_error("Failed to create user");
        }
    };

    let pool = Arc::new(state.db_pool().clone());
    let repo = AdminUserRepository::new(pool);

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
            // Assign roles if provided
            if let Some(role_uuids) = &req.role_uuids {
                if let Err(e) = repo.update_user_roles(user_uuid, role_uuids).await {
                    error!("Failed to assign roles to user: {e}");
                    return ApiResponse::<()>::internal_error(
                        "User created but failed to assign roles",
                    );
                }
            }
            // Invalidate cache for the new user
            state
                .role_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            // Load role_uuids for response
            let role_uuids = repo.get_user_roles(user_uuid).await.unwrap_or_default();
            ApiResponse::<UserResponse>::created(UserResponse::from_with_roles(&user, &role_uuids))
        }
        Ok(None) => ApiResponse::<()>::internal_error("User created but not found"),
        Err(e) => {
            error!("Failed to fetch created user: {e}");
            ApiResponse::<()>::internal_error("User created but failed to retrieve")
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
    // Check permission - need Users:Update to update users
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Update, None)
    {
        return resp;
    }

    // Validate request
    if let Err(errors) = req.validate() {
        return ApiResponse::unprocessable_entity(&format!("Validation error: {errors}"));
    }

    let actor_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Failed to parse actor UUID from claims: {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication");
        }
    };

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

    // Update roles if provided
    if let Some(role_uuids) = &req.role_uuids {
        if let Err(e) = repo.update_user_roles(user_uuid, role_uuids).await {
            error!("Failed to update user roles: {e}");
            return ApiResponse::<()>::internal_error("Failed to update user roles");
        }
    }

    if let Some(is_active) = req.is_active {
        user.is_active = is_active;
    }

    if let Some(super_admin) = req.super_admin {
        user.super_admin = super_admin;
    }

    // Setting the status back to Active is how an operator unlocks an account;
    // set_status clears the failed-attempt counter and the lockout expiry.
    if let Some(status) = &req.status {
        user.set_status(status.clone());
    }

    // Update password if provided
    if let Some(password) = &req.password {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        use argon2::Argon2;

        // An SSO-provisioned account authenticates at the identity provider
        // and nowhere else. Letting an admin set a password on one would
        // bypass every other guard in this feature with a single PUT, and
        // reopen the local login path the provisioning flow deliberately
        // closed. Refuse loudly rather than ignore the field: an operator who
        // thinks they set a password and did not is worse off than one told no.
        if user.is_sso_provisioned {
            return ApiResponse::<()>::bad_request(
                "This user signs in through single sign-on; a password cannot be set on \
                 their account. Remove the single-sign-on link first if they should \
                 become a local account.",
            );
        }

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

    // Update user via service (handles audit logging)
    let service = state.admin_user_service();
    match service.update_user(&user, actor_uuid).await {
        Ok(()) => {
            // Invalidate cache for the updated user
            state
                .role_service()
                .invalidate_user_permissions_cache(&user_uuid)
                .await;
            // Load role_uuids for response
            let role_uuids = repo.get_user_roles(user_uuid).await.unwrap_or_default();
            ApiResponse::ok(UserResponse::from_with_roles(&user, &role_uuids))
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
    // Check permission - need Users:Delete to delete users
    if let Err(resp) =
        auth.require_permission(&ResourceNamespace::Users, &PermissionType::Delete, None)
    {
        return resp;
    }

    let actor_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            error!("Failed to parse actor UUID from claims: {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication");
        }
    };

    let user_uuid = path.into_inner();

    // Verify user is active before deleting (service checks existence)
    let service = state.admin_user_service();
    match service.get_user_by_uuid(&user_uuid).await {
        Ok(Some(user)) => {
            if !user.is_active {
                return ApiResponse::<()>::conflict("User is already inactive");
            }
        }
        Ok(None) => {
            return ApiResponse::<()>::not_found("User not found");
        }
        Err(e) => {
            error!("Failed to find user: {e}");
            return ApiResponse::<()>::internal_error("Failed to find user");
        }
    }

    // Delete via service (handles audit logging)
    match service.delete_user(&user_uuid, actor_uuid).await {
        Ok(()) => {
            // Invalidate cache for the deleted user
            state
                .role_service()
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
