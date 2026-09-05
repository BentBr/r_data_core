#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

use actix_web::{post, web, Responder};
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::auth::models::{AdminLoginRequest, AdminLoginResponse, AdminRegisterRequest};
use crate::admin::auth::rate_limit;
use crate::admin::auth::routes::helpers::{complete_login, handle_password_failure};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::OptionalAuth;
use crate::response::ApiResponse;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use validator::Validate;

/// Login endpoint for admin users
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/login",
    tag = "admin-auth",
    request_body = AdminLoginRequest,
    responses(
        (status = 200, description = "Login successful. Copy the token and click the Authorize button at the top to use it.", body = AdminLoginResponse),
        (status = 400, description = "Invalid request format or missing JSON body"),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account locked or inactive"),
        (status = 422, description = "Missing or invalid required fields"),
        (status = 429, description = "Too many login attempts"),
        (status = 500, description = "Internal server error")
    ),
    security() // Empty security means no authentication required
)]
#[post("/auth/login")]
pub async fn admin_login(
    req: actix_web::HttpRequest,
    data: web::Data<ApiStateWrapper>,
    login_req: Option<web::Json<AdminLoginRequest>>,
) -> impl Responder {
    // Check if JSON body is provided and validate
    let login_req = match login_req {
        Some(r) => {
            let inner = r.into_inner();
            if let Err(errors) = inner.validate() {
                let error_message = format!("Validation error: {errors}");
                return ApiResponse::unprocessable_entity(&error_message);
            }
            inner
        }
        None => return ApiResponse::bad_request("Missing or invalid JSON body"),
    };

    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));

    // Per-client-IP rate limit pre-check
    let rl_key = rate_limit::rate_limit_key(&req, rate_limit::Bucket::Login);
    if rate_limit::is_over_limit(data.cache_manager(), &rl_key).await {
        log::warn!("Login rate limit hit for key {rl_key}");
        return ApiResponse::too_many_requests("Too many login attempts, try again later");
    }

    log::debug!("Login attempt for username: {}", login_req.username);

    let mut user = match repo.find_by_username_or_email(&login_req.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Spend the same Argon2 time as a real account so the response delay
            // does not answer "does this username exist?".
            r_data_core_core::crypto::verify_dummy_password(&login_req.password);
            // Count username-enumeration probes against the IP, too.
            rate_limit::record_failure(data.cache_manager(), &rl_key).await;
            return ApiResponse::unauthorized("Invalid credentials");
        }
        Err(e) => {
            log::error!("Database error: {e:?}");
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // An automatic lockout that has run its course lifts itself, so a locked-out
    // admin recovers without operator help.
    if user.release_expired_lockout() {
        if let Err(e) = repo
            .update_lockout_state(&user.uuid, &user.status, user.failed_login_attempts, None)
            .await
        {
            log::error!("Failed to release expired lockout: {e:?}");
        }
    }

    // Verify the password FIRST. A wrong password returns the generic 401 —
    // identical to an unknown user — so the locked/inactive state below can only
    // be observed by someone who already holds valid credentials. This avoids
    // leaking account existence/state (username enumeration).
    if !user.verify_password(&login_req.password) {
        return handle_password_failure(user, &repo, &data, &rl_key, &login_req.username).await;
    }

    // Credentials are valid: only now is it safe to reveal a locked/inactive
    // account. Do NOT count this against the IP rate limit (the password was
    // correct — it is not a brute-force attempt) and do NOT reset the counter.
    if !user.can_login() {
        log::debug!("Login blocked: valid credentials but account is locked/inactive");
        return ApiResponse::inactive("Account locked or not active");
    }

    // Successful login clears the IP's failed-attempt counter.
    rate_limit::reset(data.cache_manager(), &rl_key).await;
    complete_login(user, repo, &data).await
}

/// Register a new admin user endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/register",
    tag = "admin-auth",
    request_body = AdminRegisterRequest,
    responses(
        (status = 201, description = "Registration successful"),
        (status = 400, description = "Invalid request format or missing JSON body"),
        (status = 403, description = "Insufficient permissions"),
        (status = 422, description = "Missing or invalid required fields"),
        (status = 429, description = "Too many registration attempts"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/auth/register")]
pub async fn admin_register(
    req: actix_web::HttpRequest,
    data: web::Data<ApiStateWrapper>,
    register_req: Option<web::Json<AdminRegisterRequest>>,
    auth: OptionalAuth,
) -> impl Responder {
    // Anonymous registration is open by design, but every request costs two
    // lookups and an Argon2 hash — throttle it per client IP like login.
    let anonymous = auth.0.is_none();
    let rl_key = rate_limit::rate_limit_key(&req, rate_limit::Bucket::Register);
    if anonymous && rate_limit::is_over_limit(data.cache_manager(), &rl_key).await {
        log::warn!("Registration rate limit hit for key {rl_key}");
        return ApiResponse::too_many_requests("Too many registration attempts, try again later");
    }

    // Check if JSON body is provided
    let Some(register_req) = register_req else {
        return ApiResponse::bad_request("Missing or invalid JSON body");
    };

    // Validate the request data using the Validate trait
    let register_req = register_req.into_inner();
    if let Err(errors) = register_req.validate() {
        // Format validation errors into a readable message
        let error_message = format!("Validation error: {errors}");
        return ApiResponse::unprocessable_entity(&error_message);
    }

    if anonymous {
        rate_limit::record_failure(data.cache_manager(), &rl_key).await;
    }

    // Get authentication info from the OptionalAuth extractor
    let (is_authenticated, creator_uuid) = auth.0.as_ref().map_or_else(
        || (false, Uuid::nil()),
        |claims| {
            // Extract the UUID
            let creator = match Uuid::parse_str(&claims.sub) {
                Ok(uuid) => uuid,
                Err(e) => {
                    log::error!(
                        "Failed to parse UUID from claims.sub: {}, error: {}",
                        claims.sub,
                        e
                    );
                    Uuid::nil()
                }
            };
            (true, creator)
        },
    );

    // Create repository
    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));

    // Check if a username or email already exists
    match repo.find_by_username_or_email(&register_req.username).await {
        Ok(Some(_)) => {
            // Don't reveal that a username exists, just return a success response
            // This prevents user enumeration attacks
            return ApiResponse::created_message("User registration processed");
        }
        Ok(None) => false,
        Err(e) => {
            log::error!("Error checking for existing user: {e:?}");
            return ApiResponse::internal_error("Registration failed");
        }
    };

    // Also check by email
    match repo.find_by_username_or_email(&register_req.email).await {
        Ok(Some(_)) => {
            // Don't reveal that email exists, just return a success response
            // This prevents user enumeration attacks
            return ApiResponse::created_message("User registration processed");
        }
        Ok(None) => false,
        Err(e) => {
            log::error!("Error checking for existing email: {e:?}");
            return ApiResponse::internal_error("Registration failed");
        }
    };

    // Create the user
    let params = r_data_core_persistence::CreateAdminUserParams {
        username: &register_req.username,
        email: &register_req.email,
        password: &register_req.password,
        first_name: &register_req.first_name,
        last_name: &register_req.last_name,
        role: register_req.role.as_deref(),
        is_active: is_authenticated,
        creator_uuid,
    };
    let result = repo.create_admin_user(&params).await;

    match result {
        Ok(uuid) => {
            if is_authenticated {
                ApiResponse::ok(serde_json::json!({
                    "message": "User registration processed successfully. User is active and published.",
                    "uuid": uuid.to_string(),
                    "is_authenticated": is_authenticated,
                    "creator_uuid": creator_uuid.to_string()
                }))
            } else {
                ApiResponse::ok(serde_json::json!({
                    "message": "User registration processed successfully. User must be activated by an admin.",
                    "uuid": uuid.to_string(),
                    "is_authenticated": is_authenticated,
                    "creator_uuid": creator_uuid.to_string()
                }))
            }
        }
        Err(e) => {
            // Log the detailed error for debugging
            log::error!("User registration failed: {e:?}");
            ApiResponse::internal_error("Registration failed")
        }
    }
}
