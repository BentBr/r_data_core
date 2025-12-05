#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use r_data_core_core::admin_user::AdminUser;

/// User response DTO (for API serialization)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    /// User UUID
    pub uuid: Uuid,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Full name
    pub full_name: String,
    /// First name
    pub first_name: Option<String>,
    /// Last name
    pub last_name: Option<String>,
    /// User role
    pub role: String,
    /// User account status
    pub status: String,
    /// Whether user is active
    pub is_active: bool,
    /// Whether user is admin
    pub is_admin: bool,
    /// Super admin flag
    pub super_admin: bool,
    /// Last login time
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_login: Option<OffsetDateTime>,
    /// Failed login attempts
    pub failed_login_attempts: i32,
    /// When the user was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the user was last updated
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// UUID of the user who created this user
    pub created_by: Uuid,
}

impl From<&AdminUser> for UserResponse {
    fn from(user: &AdminUser) -> Self {
        Self {
            uuid: user.uuid,
            username: user.username.clone(),
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: user.role.as_str().to_string(),
            status: format!("{:?}", user.status),
            is_active: user.is_active,
            is_admin: user.is_admin,
            super_admin: user.super_admin,
            last_login: user.last_login,
            failed_login_attempts: user.failed_login_attempts,
            created_at: user.created_at,
            updated_at: user.updated_at,
            created_by: user.base.created_by,
        }
    }
}

/// Create user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateUserRequest {
    /// Username
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    /// Email address
    #[validate(email)]
    pub email: String,
    /// Password
    #[validate(length(min = 8))]
    pub password: String,
    /// First name
    pub first_name: String,
    /// Last name
    pub last_name: String,
    /// User role (optional, defaults to Custom("Default"))
    pub role: Option<String>,
    /// Whether user is active
    pub is_active: Option<bool>,
    /// Super admin flag
    pub super_admin: Option<bool>,
}

/// Update user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateUserRequest {
    /// Email address (optional)
    #[validate(email)]
    pub email: Option<String>,
    /// Password (optional, only set if provided)
    #[validate(length(min = 8))]
    pub password: Option<String>,
    /// First name (optional)
    pub first_name: Option<String>,
    /// Last name (optional)
    pub last_name: Option<String>,
    /// User role (optional)
    pub role: Option<String>,
    /// Whether user is active (optional)
    pub is_active: Option<bool>,
    /// Super admin flag (optional)
    pub super_admin: Option<bool>,
}
