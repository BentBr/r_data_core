use crate::api::auth::auth_enum;
use actix_web::{post, web, HttpMessage, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::api::jwt::{
    generate_access_token, verify_jwt, AuthUserClaims, ACCESS_TOKEN_EXPIRY_SECONDS,
    REFRESH_TOKEN_EXPIRY_SECONDS,
};
use crate::api::response::ApiResponse;
use crate::api::ApiState;
use crate::entity::admin_user::repository::AdminUserRepository;
use crate::entity::admin_user::repository_trait::AdminUserRepositoryTrait;
use crate::entity::refresh_token::{
    RefreshToken, RefreshTokenRepository, RefreshTokenRepositoryTrait,
};
use std::sync::Arc;
use utoipa::ToSchema;

/// Empty request body for endpoints that don't require any input
#[derive(Debug, Deserialize, ToSchema)]
pub struct EmptyRequest {}

/// Refresh token request body
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Refresh token response body
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,

    /// New refresh token
    pub refresh_token: String,

    /// Access token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub access_expires_at: OffsetDateTime,

    /// Refresh token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub refresh_expires_at: OffsetDateTime,
}

/// Request to logout with refresh token
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LogoutRequest {
    /// Refresh token to revoke
    pub refresh_token: String,
}

/// Admin login request body
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct AdminLoginRequest {
    /// Username or email
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    /// Password
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

/// Admin login response body
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminLoginResponse {
    /// JWT access token
    pub access_token: String,

    /// Refresh token
    pub refresh_token: String,

    /// User UUID
    pub user_uuid: String,

    /// Username
    pub username: String,

    /// User role
    pub role: String,

    /// Access token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub access_expires_at: OffsetDateTime,

    /// Refresh token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub refresh_expires_at: OffsetDateTime,
}

/// Validate email format
fn validate_email(email: &str) -> Result<(), ValidationError> {
    if !email.contains('@') || !email.contains('.') {
        let mut err = ValidationError::new("invalid_email");
        err.message = Some("Invalid email format".into());
        return Err(err);
    }
    Ok(())
}

/// Admin registration request body
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AdminRegisterRequest {
    /// Username
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    /// Email
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// Password
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    /// First name
    #[validate(length(min = 1, message = "First name is required"))]
    pub first_name: String,

    /// Last name
    #[validate(length(min = 1, message = "Last name is required"))]
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

/// Helper to get authenticated user info if available
fn get_auth_info(req: &HttpRequest) -> (bool, Uuid) {
    log::debug!("get_auth_info called");

    // Check for Authorization header first
    let auth_header = req.headers().get("Authorization");
    log::debug!("Authorization header present: {}", auth_header.is_some());

    let has_auth_header = if let Some(header) = auth_header {
        if let Ok(auth_str) = header.to_str() {
            log::debug!("Auth header value: {}", auth_str);
            auth_str.starts_with("Bearer ")
        } else {
            log::error!("Failed to convert auth header to string");
            false
        }
    } else {
        false
    };

    // Check if this is an authenticated request through extensions (middleware added it)
    let auth_user = req.extensions().get::<AuthUserClaims>().cloned();
    log::debug!("AuthUserClaims in extensions: {}", auth_user.is_some());
    if let Some(ref claims) = auth_user {
        log::debug!("Claims content: {:?}", claims);
    }

    let is_authenticated = auth_user.is_some() || has_auth_header;

    log::debug!("Auth header present: {}", has_auth_header);
    log::debug!("Extensions has auth: {}", auth_user.is_some());
    log::debug!("Is authenticated: {}", is_authenticated);

    // Get creator UUID if authenticated
    let creator_uuid = if let Some(claims) = auth_user {
        // The sub claim is now the UUID string directly
        log::debug!("Parsing UUID from claims.sub: {}", claims.sub);
        match Uuid::parse_str(&claims.sub) {
            Ok(uuid) => {
                log::debug!("Using UUID from claims.sub: {}", uuid);
                uuid
            }
            Err(e) => {
                log::error!(
                    "Failed to parse UUID from claims.sub: {}, error: {}",
                    claims.sub,
                    e
                );
                Uuid::nil()
            }
        }
    } else if has_auth_header {
        log::debug!("No claims in extensions, trying to extract from auth header");
        // If we have an auth header but no claims in extensions, try to extract the token and verify it manually
        if let Some(header) = auth_header {
            if let Ok(auth_str) = header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..]; // Remove "Bearer " prefix
                    log::debug!("Extracted token from header: {}", token);

                    // Get JWT secret from app state
                    if let Some(state) = req.app_data::<web::Data<ApiState>>() {
                        let jwt_secret = &state.jwt_secret;
                        log::debug!("Got JWT secret, length: {}", jwt_secret.len());

                        // Verify JWT token
                        match verify_jwt(token, jwt_secret) {
                            Ok(claims) => {
                                log::debug!("Manually verified JWT token, claims: {:?}", claims);
                                // Extract the UUID
                                if let Ok(uuid) = Uuid::parse_str(&claims.sub) {
                                    log::debug!("Manually extracted UUID from token: {}", uuid);
                                    return (true, uuid);
                                } else {
                                    log::error!(
                                        "Failed to parse UUID from claims.sub: {}",
                                        claims.sub
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to verify JWT: {:?}", e);
                            }
                        }
                    } else {
                        log::error!("Failed to get ApiState from request");
                    }
                }
            }
        }

        log::debug!("Could not extract UUID from auth header, using nil UUID");
        Uuid::nil()
    } else {
        log::debug!("No auth claims found, using nil UUID");
        Uuid::nil() // Use nil UUID for unauthenticated requests
    };

    log::debug!(
        "Final result - is_authenticated: {}, creator_uuid: {}",
        is_authenticated,
        creator_uuid
    );
    (is_authenticated, creator_uuid)
}

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
    data: web::Data<ApiState>,
    login_req: Option<web::Json<AdminLoginRequest>>,
) -> impl Responder {
    // Check if JSON body is provided
    let login_req = match login_req {
        Some(req) => req,
        None => {
            return ApiResponse::bad_request("Missing or invalid JSON body");
        }
    };

    // Validate the request data using the Validate trait
    if let Err(errors) = login_req.validate() {
        // Format validation errors into a readable message
        let error_message = format!("Validation error: {}", errors);
        return ApiResponse::unprocessable_entity(&error_message);
    }

    // Create repository
    let pool = Arc::new(data.db_pool.clone());
    let repo = Arc::new(AdminUserRepository::new(pool));

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

    // Generate short-lived access token (30 minutes)
    let access_token = match generate_access_token(&user, &data.jwt_secret) {
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
    let refresh_repo = RefreshTokenRepository::new(data.db_pool.clone());
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
    data: web::Data<ApiState>,
    register_req: Option<web::Json<AdminRegisterRequest>>,
    auth: auth_enum::OptionalAuth,
) -> impl Responder {
    // Check if JSON body is provided
    let register_req = match register_req {
        Some(req) => req,
        None => {
            return ApiResponse::bad_request("Missing or invalid JSON body");
        }
    };

    // Validate the request data using the Validate trait
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
    let pool = Arc::new(data.db_pool.clone());
    let repo = Arc::new(AdminUserRepository::new(pool));

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
    data: web::Data<ApiState>,
    request: web::Json<LogoutRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool.clone());

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
    data: web::Data<ApiState>,
    request: web::Json<RefreshTokenRequest>,
) -> impl Responder {
    let refresh_repo = RefreshTokenRepository::new(data.db_pool.clone());
    let admin_repo = AdminUserRepository::new(data.db_pool.clone().into());

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
    let new_access_token = match generate_access_token(&user, &data.jwt_secret) {
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
    data: web::Data<ApiState>,
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

    let refresh_repo = RefreshTokenRepository::new(data.db_pool.clone());

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
