#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{post, web, HttpMessage, HttpRequest, Responder};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::OptionalAuth;
use crate::jwt::{
    generate_access_token, AuthUserClaims, ACCESS_TOKEN_EXPIRY_SECONDS,
    REFRESH_TOKEN_EXPIRY_SECONDS,
};
use crate::response::ApiResponse;
use r_data_core_core::admin_user::UserRole;
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_persistence::{RefreshTokenRepository, RefreshTokenRepositoryTrait};

use crate::admin::auth::models::{
    AdminLoginRequest, AdminLoginResponse, AdminRegisterRequest, LogoutRequest,
    RefreshTokenRequest, RefreshTokenResponse,
};
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
        (status = 500, description = "Internal server error")
    ),
    security() // Empty security means no authentication required
)]
#[post("/auth/login")]
pub async fn admin_login(
    data: web::Data<ApiStateWrapper>,
    login_req: Option<web::Json<AdminLoginRequest>>,
) -> impl Responder {
    // Check if JSON body is provided and validate
    let login_req = match login_req {
        Some(req) => {
            let inner = req.into_inner();
            // Validate the request data using the Validate trait
            if let Err(errors) = inner.validate() {
                // Format validation errors into a readable message
                let error_message = format!("Validation error: {}", errors);
                return ApiResponse::unprocessable_entity(&error_message);
            }
            inner
        }
        None => {
            return ApiResponse::bad_request("Missing or invalid JSON body");
        }
    };

    // Create repository
    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));

    // Debug: Log the login attempt
    log::debug!("Login attempt for username: {}", login_req.username);

    // Find the user
    let user_result = repo.find_by_username_or_email(&login_req.username).await;

    // Handle database error
    let user = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Don't reveal if user exists or not
            return ApiResponse::unauthorized("Invalid credentials");
        }
        Err(e) => {
            log::error!("Database error: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Verify password
    if !user.verify_password(&login_req.password) {
        // Log failed attempt but don't reveal specific error
        log::debug!(
            "Password verification failed for user: {}",
            login_req.username
        );
        return ApiResponse::unauthorized("Invalid credentials");
    }

    // Check if user is active
    if !user.is_active {
        log::debug!("User account is inactive: {}", user.username);
        return ApiResponse::inactive("Account not active");
    }

    // Update last login time
    if let Err(e) = repo.update_last_login(&user.uuid).await {
        // Log the error but continue with authentication
        log::error!("Failed to update last login: {:?}", e);
    }

    // Load all permission schemes for user (if not SuperAdmin or super_admin)
    let schemes = if user.super_admin || matches!(user.role, UserRole::SuperAdmin) {
        // SuperAdmin or super_admin doesn't need schemes - handled in JWT generation
        vec![]
    } else {
        // Load all user's permission schemes
        match data
            .permission_scheme_service()
            .get_schemes_for_user(user.uuid, &repo)
            .await
        {
            Ok(s) => {
                log::debug!(
                    "Loaded {} permission schemes for user {}",
                    s.len(),
                    user.username
                );
                s
            }
            Err(e) => {
                log::warn!("Failed to load permission schemes for user: {}", e);
                vec![]
            }
        }
    };

    // Generate short-lived access token (30 minutes)
    // Use short-lived expiration for access tokens, but get secret from config
    let mut access_token_config = data.api_config().clone();
    access_token_config.jwt_expiration = ACCESS_TOKEN_EXPIRY_SECONDS;
    let access_token = match generate_access_token(&user, &access_token_config, &schemes) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate access token: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Generate refresh token
    let refresh_token = RefreshToken::generate_token();
    let refresh_token_hash = match RefreshToken::hash_token(&refresh_token) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash refresh token: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Calculate expiration times
    let access_expires_at = OffsetDateTime::now_utc()
        .checked_add(Duration::seconds(ACCESS_TOKEN_EXPIRY_SECONDS as i64)) // 30 minutes
        .unwrap_or(OffsetDateTime::now_utc());

    let refresh_expires_at = OffsetDateTime::now_utc()
        .checked_add(Duration::seconds(REFRESH_TOKEN_EXPIRY_SECONDS as i64))
        .unwrap_or(OffsetDateTime::now_utc());

    // Store refresh token in database
    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());
    let device_info = Some(serde_json::json!({
        "user_agent": "login",
        "login_time": OffsetDateTime::now_utc()
    }));

    if let Err(e) = refresh_repo
        .create(
            user.uuid,
            refresh_token_hash,
            refresh_expires_at,
            device_info,
            // IP address would be extracted from request in real implementation
        )
        .await
    {
        log::error!("Failed to store refresh token: {:?}", e);
        return ApiResponse::internal_error("Authentication failed");
    }

    // Build response
    let response = AdminLoginResponse {
        access_token,
        refresh_token,
        user_uuid: user.uuid.to_string(),
        username: user.username,
        role: format!("{:?}", user.role),
        access_expires_at,
        refresh_expires_at,
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
        (status = 400, description = "Invalid request format or missing JSON body"),
        (status = 403, description = "Insufficient permissions"),
        (status = 422, description = "Missing or invalid required fields"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/auth/register")]
pub async fn admin_register(
    data: web::Data<ApiStateWrapper>,
    register_req: Option<web::Json<AdminRegisterRequest>>,
    auth: OptionalAuth,
) -> impl Responder {
    // Check if JSON body is provided
    let register_req = match register_req {
        Some(req) => req,
        None => {
            return ApiResponse::bad_request("Missing or invalid JSON body");
        }
    };

    // Validate the request data using the Validate trait
    let register_req = register_req.into_inner();
    if let Err(errors) = register_req.validate() {
        // Format validation errors into a readable message
        let error_message = format!("Validation error: {}", errors);
        return ApiResponse::unprocessable_entity(&error_message);
    }

    // Get authentication info from the OptionalAuth extractor
    let (is_authenticated, creator_uuid) = match &auth.0 {
        Some(claims) => {
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
        }
        None => (false, Uuid::nil()),
    };

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
            log::error!("Error checking for existing user: {:?}", e);
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
            log::error!("Error checking for existing email: {:?}", e);
            return ApiResponse::internal_error("Registration failed");
        }
    };

    // Create the user
    let result = repo
        .create_admin_user(
            &register_req.username,
            &register_req.email,
            &register_req.password,
            &register_req.first_name,
            &register_req.last_name,
            register_req.role.as_deref(),
            is_authenticated,
            creator_uuid,
        )
        .await;

    match result {
        Ok(uuid) => {
            if is_authenticated {
                ApiResponse::ok(serde_json::json!({
                    "message": format!("User registration processed successfully. User is active and published."),
                    "uuid": uuid.to_string(),
                    "is_authenticated": is_authenticated,
                    "creator_uuid": creator_uuid.to_string()
                }))
            } else {
                ApiResponse::ok(serde_json::json!({
                    "message": format!("User registration processed successfully. User must be activated by an admin."),
                    "uuid": uuid.to_string(),
                    "is_authenticated": is_authenticated,
                    "creator_uuid": creator_uuid.to_string()
                }))
            }
        }
        Err(e) => {
            // Log the detailed error for debugging
            log::error!("User registration failed: {:?}", e);
            ApiResponse::internal_error("Registration failed")
        }
    }
}

/// Logout endpoint for admin users
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/logout",
    tag = "admin-auth",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful"),
        (status = 400, description = "Invalid request format"),
        (status = 401, description = "Invalid refresh token"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/logout")]
pub async fn admin_logout(
    data: web::Data<ApiStateWrapper>,
    request: web::Json<LogoutRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());

    // Hash the provided refresh token
    let token_hash = match RefreshToken::hash_token(&request.refresh_token) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash refresh token for logout: {:?}", e);
            return ApiResponse::bad_request("Invalid token format");
        }
    };

    // Revoke the refresh token
    match refresh_repo.revoke_by_token_hash(&token_hash).await {
        Ok(_) => {
            log::info!("User logged out successfully, refresh token revoked");
            ApiResponse::message("Logout successful")
        }
        Err(e) => {
            log::error!("Failed to revoke refresh token during logout: {:?}", e);
            ApiResponse::internal_error("Logout failed")
        }
    }
}

/// Refresh access token endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/refresh",
    tag = "admin-auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = RefreshTokenResponse),
        (status = 400, description = "Invalid request format"),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/auth/refresh")]
pub async fn admin_refresh_token(
    data: web::Data<ApiStateWrapper>,
    request: web::Json<RefreshTokenRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());
    let admin_repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));

    // Hash the provided refresh token
    let token_hash = match RefreshToken::hash_token(&request.refresh_token) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash refresh token: {:?}", e);
            return ApiResponse::unauthorized("Invalid refresh token");
        }
    };

    // Find the refresh token in database
    let refresh_token = match refresh_repo.find_by_token_hash(&token_hash).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            log::warn!("Refresh token not found");
            return ApiResponse::unauthorized("Invalid refresh token");
        }
        Err(e) => {
            log::error!("Database error finding refresh token: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Check if token is valid
    if !refresh_token.is_valid() {
        log::warn!("Refresh token is expired or revoked");
        return ApiResponse::unauthorized("Refresh token expired or revoked");
    }

    // Get the user
    let user = match admin_repo.find_by_uuid(&refresh_token.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            log::error!("User not found for refresh token");
            return ApiResponse::unauthorized("Invalid refresh token");
        }
        Err(e) => {
            log::error!("Database error finding user: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Check if user is still active
    if !user.is_active {
        log::warn!(
            "Attempt to refresh token for inactive user: {}",
            user.username
        );
        return ApiResponse::unauthorized("Account not active");
    }

    // Generate new access token
    // Load all permission schemes for user (if not SuperAdmin or super_admin)
    let schemes = if user.super_admin || matches!(user.role, UserRole::SuperAdmin) {
        // SuperAdmin or super_admin doesn't need schemes - handled in JWT generation
        vec![]
    } else {
        // Load all user's permission schemes
        match data
            .permission_scheme_service()
            .get_schemes_for_user(user.uuid, &admin_repo)
            .await
        {
            Ok(s) => {
                log::debug!(
                    "Loaded {} permission schemes for user {} during token refresh",
                    s.len(),
                    user.username
                );
                s
            }
            Err(e) => {
                log::warn!("Failed to load permission schemes for user: {}", e);
                vec![]
            }
        }
    };

    // Use short-lived expiration for access tokens, but get secret from config
    let mut access_token_config = data.api_config().clone();
    access_token_config.jwt_expiration = ACCESS_TOKEN_EXPIRY_SECONDS;
    let new_access_token = match generate_access_token(&user, &access_token_config, &schemes) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate new access token: {:?}", e);
            return ApiResponse::internal_error("Token refresh failed");
        }
    };

    // Generate new refresh token
    let new_refresh_token_string = RefreshToken::generate_token();
    let new_refresh_token_hash = match RefreshToken::hash_token(&new_refresh_token_string) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Failed to hash new refresh token: {:?}", e);
            return ApiResponse::internal_error("Token refresh failed");
        }
    };

    // Calculate expiration times
    let access_expires_at = OffsetDateTime::now_utc()
        .checked_add(Duration::seconds(ACCESS_TOKEN_EXPIRY_SECONDS as i64)) // 30 minutes
        .unwrap_or(OffsetDateTime::now_utc());

    let refresh_expires_at = OffsetDateTime::now_utc()
        .checked_add(Duration::seconds(REFRESH_TOKEN_EXPIRY_SECONDS as i64))
        .unwrap_or(OffsetDateTime::now_utc());

    // Update the old refresh token as used
    if let Err(e) = refresh_repo.update_last_used(refresh_token.id).await {
        log::error!("Failed to update refresh token last used: {:?}", e);
    }

    // Create new refresh token in database
    let device_info = refresh_token.device_info.clone();
    if let Err(e) = refresh_repo
        .create(
            user.uuid,
            new_refresh_token_hash,
            refresh_expires_at,
            device_info,
        )
        .await
    {
        log::error!("Failed to store new refresh token: {:?}", e);
        return ApiResponse::internal_error("Token refresh failed");
    }

    // Revoke the old refresh token
    if let Err(e) = refresh_repo.revoke_by_id(refresh_token.id).await {
        log::error!("Failed to revoke old refresh token: {:?}", e);
        // Continue anyway since new token was created
    }

    // Build response
    let response = RefreshTokenResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token_string,
        access_expires_at,
        refresh_expires_at,
    };

    ApiResponse::ok(response)
}

/// Revoke all refresh tokens for current user endpoint
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/revoke-all",
    tag = "admin-auth",
    responses(
        (status = 200, description = "All tokens revoked successfully"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/auth/revoke-all")]
pub async fn admin_revoke_all_tokens(
    data: web::Data<ApiStateWrapper>,
    req: HttpRequest,
) -> impl Responder {
    // Extract user claims from JWT
    let extensions = req.extensions();
    let claims = match extensions.get::<AuthUserClaims>() {
        Some(claims) => claims,
        None => {
            return ApiResponse::unauthorized("Authentication required");
        }
    };

    let user_uuid = match Uuid::parse_str(&claims.sub) {
        Ok(uuid) => uuid,
        Err(_) => {
            return ApiResponse::unauthorized("Invalid user ID in token");
        }
    };

    let refresh_repo = RefreshTokenRepository::new(data.db_pool().clone());

    // Revoke all refresh tokens for the user
    match refresh_repo.revoke_all_for_user(user_uuid).await {
        Ok(count) => {
            log::info!("Revoked {} refresh tokens for user {}", count, claims.name);
            ApiResponse::ok(format!("Revoked {} active sessions", count))
        }
        Err(e) => {
            log::error!(
                "Failed to revoke all tokens for user {}: {:?}",
                claims.name,
                e
            );
            ApiResponse::internal_error("Failed to revoke tokens")
        }
    }
}

/// Register auth routes
pub fn register_routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(admin_login)
        .service(admin_register)
        .service(admin_logout)
        .service(admin_refresh_token)
        .service(admin_revoke_all_tokens);
}
