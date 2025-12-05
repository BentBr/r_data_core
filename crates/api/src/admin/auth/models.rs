#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use validator::Validate;

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

    /// Access token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub access_expires_at: OffsetDateTime,

    /// Refresh token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    pub refresh_expires_at: OffsetDateTime,
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
