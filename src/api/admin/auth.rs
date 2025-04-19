use crate::api::auth::auth_enum;
use actix_web::{post, web, HttpMessage, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::api::jwt::{generate_jwt, verify_jwt, AuthUserClaims};
use crate::api::response::ApiResponse;
use crate::api::ApiState;
use crate::repository::admin_user_repository::{AdminUserRepository, PgAdminUserRepository};
use utoipa::ToSchema;

/// Empty request body for endpoints that don't require any input
#[derive(Debug, Deserialize, ToSchema)]
pub struct EmptyRequest {}

/// Admin login request body
#[derive(Debug, Deserialize, ToSchema, Validate)]
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
    /// JWT token
    pub token: String,

    /// User UUID
    pub user_uuid: String,

    /// Username
    pub username: String,

    /// User role
    pub role: String,

    /// Token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
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
    let repo = PgAdminUserRepository::new(data.db_pool.clone());

    // Debug: Log the login attempt
    log::debug!("Login attempt for username: {}", login_req.username);

    // Find user by username or email
    let user_result = repo.find_by_username_or_email(&login_req.username).await;

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
    let password_valid = user.verify_password(&login_req.password);

    if !password_valid {
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

    // Generate JWT token (30 day expiration for admin)
    let token = match generate_jwt(&user, &data.jwt_secret, 2592000) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate JWT: {:?}", e);
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // Calculate expiration time
    let expires_at = OffsetDateTime::now_utc()
        .checked_add(Duration::seconds(2592000))
        .unwrap_or(OffsetDateTime::now_utc());

    // Update last login time using repository
    if let Err(_) = repo.update_last_login(&user.uuid).await {
        // Continue even if update fails, just log it in a real implementation
    }

    // Build response
    let response = AdminLoginResponse {
        token,
        user_uuid: user.uuid.to_string(),
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
    let repo = PgAdminUserRepository::new(data.db_pool.clone());

    // Check if username or email already exists - don't leak this info in response
    let existing_user = repo.find_by_username_or_email(&register_req.username).await;
    if let Ok(Some(_)) = existing_user {
        // Don't reveal that username exists, just return success response
        // This prevents user enumeration attacks
        return ApiResponse::created_message("User registration processed");
    }

    // Attempt to create the user
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
    responses(
        (status = 200, description = "Logout successful"),
        (status = 400, description = "Invalid request format"),
        (status = 401, description = "Unauthorized, invalid or missing token"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/auth/logout")]
pub async fn admin_logout(
    _body: web::Json<EmptyRequest>,
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    log::debug!("Logout successful for user: {}", auth.0.name);

    // Acknowledge the logout
    log::debug!("Sending logout successful response");
    ApiResponse::message("Logout successful")
}
