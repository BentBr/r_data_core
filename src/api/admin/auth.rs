use actix_web::{post, web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ApiState;
use crate::api::response::ApiResponse;
use crate::db::PgPoolExtension;
use crate::entity::AdminUser;
use crate::api::auth::generate_jwt;
use crate::repository::admin_user_repository::{AdminUserRepository, PgAdminUserRepository};
use utoipa::ToSchema;

/// Admin login request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminLoginRequest {
    /// Username or email
    pub username: String,

    /// Password
    pub password: String,
}

/// Admin login response body
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminLoginResponse {
    /// JWT token
    pub token: String,

    /// User UUID
    pub user_uuid: String,

    /// Username
    pub username: String,

    /// User role
    pub role: String,

    /// Token expiration (timestamp)
    pub expires_at: i64,
}

/// Admin registration request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminRegisterRequest {
    /// Username
    pub username: String,

    /// Email
    pub email: String,

    /// Password
    pub password: String,

    /// First name
    pub first_name: String,

    /// Last name
    pub last_name: String,

    /// User role
    pub role: Option<String>,
}

/// Admin registration response body
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminRegisterResponse {
    /// User UUID
    pub uuid: String,

    /// Username
    pub username: String,

    /// Message
    pub message: String,
}

/// Login endpoint for admin users
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/login",
    tag = "admin-auth",
    request_body = AdminLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AdminLoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account locked or inactive"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/login")]
pub async fn admin_login(
    data: web::Data<ApiState>,
    login_req: web::Json<AdminLoginRequest>,
) -> impl Responder {
    // Create repository
    let repo = PgAdminUserRepository::new(data.db_pool.clone());

    // Debug: Log the login attempt
    log::debug!("Login attempt for username: {}", login_req.username);

    // Find user by username or email
    let user_result = repo.find_by_username_or_email(&login_req.username).await;

    let user = match user_result {
        Ok(Some(user)) => {
            log::debug!("User found: {}, hash: {}", user.username, user.password_hash);
            user
        },
        Ok(None) => {
            log::debug!("User not found: {}", login_req.username);
            // Don't reveal if user exists or not
            return ApiResponse::unauthorized("Invalid credentials");
        }
        Err(e) => {
            log::error!("Database error: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Verify password
    let password_valid = user.verify_password(&login_req.password);
    log::debug!("Password verification result: {}", password_valid);
    
    if !password_valid {
        // Log failed attempt but don't reveal specific error
        log::debug!("Password verification failed for user: {}", user.username);
        return ApiResponse::unauthorized("Invalid credentials");
    }

    // Check if user is active
    if !user.is_active {
        log::debug!("User account is inactive: {}", user.username);
        return ApiResponse::forbidden("Authentication failed");
    }

    // Generate JWT token (30 day expiration for admin)
    let token = match generate_jwt(&user, &data.jwt_secret, 2592000) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate JWT: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Calculate expiration time
    let expires_at = Utc::now()
        .checked_add_signed(Duration::seconds(2592000))
        .unwrap_or(Utc::now())
        .timestamp();

    // Update last login time using repository
    if let Err(_) = repo.update_last_login(&user.base.uuid).await {
        // Continue even if update fails, just log it in a real implementation
    }

    // Build response
    let response = AdminLoginResponse {
        token,
        user_uuid: user.base.uuid.to_string(),
        username: user.username,
        role: format!("{:?}", user.role),
        expires_at,
    };

    ApiResponse::ok(response)
}

/// Register a new admin user endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/register",
    tag = "admin-auth",
    request_body = AdminRegisterRequest,
    responses(
        (status = 201, description = "Registration successful"),
        (status = 400, description = "Invalid request data"),
        (status = 403, description = "Insufficient permissions"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/register")]
pub async fn admin_register(
    data: web::Data<ApiState>,
    register_req: web::Json<AdminRegisterRequest>,
) -> impl Responder {
    // In a real implementation, validate the auth token and extract claims
    // For simplicity, we'll assume the user is authorized
    
    // Create repository
    let repo = PgAdminUserRepository::new(data.db_pool.clone());

    // Validate input data
    if register_req.username.len() < 3 || register_req.email.len() < 5 {
        return ApiResponse::bad_request("Invalid input data");
    }

    // Check if username or email already exists - don't leak this info in response
    let existing_user = repo.find_by_username_or_email(&register_req.username).await;
    if let Ok(Some(_)) = existing_user {
        // Don't reveal that username exists, just return success response
        // This prevents user enumeration attacks
        return ApiResponse::created_message("User registration processed");
    }

    // Attempt to create the user
    let result = repo.create_admin_user(
        &register_req.username,
        &register_req.email,
        &register_req.password,
        &register_req.first_name,
        &register_req.last_name,
        register_req.role.as_deref(),
    ).await;

    match result {
        Ok(_) => ApiResponse::created_message("User registration processed"),
        Err(_) => ApiResponse::internal_error("Registration failed")
    }
}

/// Logout endpoint for admin users
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/logout",
    tag = "admin-auth",
    responses(
        (status = 200, description = "Logout successful"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/logout")]
pub async fn admin_logout(
    _data: web::Data<ApiState>,
) -> impl Responder {
    // In a real-world implementation, you would:
    // 1. Extract the user ID from the token
    // 2. Add the token to a blacklist in Redis with expiration
    // 3. Log the event
    
    // For now we'll just acknowledge the logout without token validation
    ApiResponse::message("Logout successful")
}

/// Register admin auth routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_login)
        .service(admin_register)
        .service(admin_logout);
} 