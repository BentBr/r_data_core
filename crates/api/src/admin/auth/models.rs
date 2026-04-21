#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::validation::patterns::EMAIL_RE;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;
use validator::Validate;

/// Empty request body for endpoints that don't require any input
#[derive(Debug, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct EmptyRequest {}

/// Refresh token request body
#[derive(Debug, Deserialize, Serialize, ToSchema, TS)]
#[ts(export)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Refresh token response body
#[derive(Debug, Serialize, ToSchema, TS)]
#[ts(export)]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,

    /// New refresh token
    pub refresh_token: String,

    /// Access token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub access_expires_at: OffsetDateTime,

    /// Refresh token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub refresh_expires_at: OffsetDateTime,
}

/// Request to logout with refresh token
#[derive(Debug, Deserialize, Validate, ToSchema, TS)]
#[ts(export)]
pub struct LogoutRequest {
    /// Refresh token to revoke
    pub refresh_token: String,
}

/// Admin login request body
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate, TS)]
#[ts(export)]
pub struct AdminLoginRequest {
    /// Username or email
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    /// Password
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

/// Admin login response body
#[derive(Debug, Serialize, ToSchema, TS)]
#[ts(export)]
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
    #[ts(type = "string")]
    pub access_expires_at: OffsetDateTime,

    /// Refresh token expiration (RFC3339 timestamp)
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub refresh_expires_at: OffsetDateTime,

    /// Whether the default admin password is still in use (false if check is disabled)
    pub using_default_password: bool,
}

/// Admin registration request body
#[derive(Debug, Deserialize, ToSchema, Validate, TS)]
#[ts(export)]
pub struct AdminRegisterRequest {
    /// Username
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    /// Email
    #[validate(regex(path = *EMAIL_RE, message = "Invalid email format"))]
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
#[derive(Debug, Serialize, ToSchema, TS)]
#[ts(export)]
pub struct AdminRegisterResponse {
    /// User UUID
    pub uuid: String,

    /// Username
    pub username: String,

    /// Message
    pub message: String,
}

/// Forgot password request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    /// Email address to send password reset link to
    pub email: String,
}

/// Reset password request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    /// Password reset token
    pub token: String,

    /// New password
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use r_data_core_core::validation::constraints;

    #[test]
    fn validation_constants_match_attributes() {
        assert_eq!(3, constraints::USERNAME_MIN_LENGTH);
        assert_eq!(8, constraints::PASSWORD_MIN_LENGTH);
        assert_eq!(1, constraints::NAME_MIN_LENGTH);
    }
}
