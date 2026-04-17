#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::validation::patterns::EMAIL_RE;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use r_data_core_core::admin_user::AdminUser;

/// User response DTO (for API serialization)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct UserResponse {
    /// User UUID
    #[ts(type = "string")]
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
    /// Role UUIDs assigned to this user
    #[ts(type = "string[]")]
    pub role_uuids: Vec<Uuid>,
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
    #[ts(type = "string | null")]
    pub last_login: Option<OffsetDateTime>,
    /// Failed login attempts
    pub failed_login_attempts: i32,
    /// When the user was created
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    /// When the user was last updated
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
    /// UUID of the user who created this user
    #[ts(type = "string")]
    pub created_by: Uuid,
}

impl UserResponse {
    /// Create a `UserResponse` from an `AdminUser` with role UUIDs
    #[must_use]
    pub fn from_with_roles(user: &AdminUser, role_uuids: &[Uuid]) -> Self {
        Self {
            uuid: user.uuid,
            username: user.username.clone(),
            email: user.email.clone(),
            full_name: user.full_name.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role_uuids: role_uuids.to_vec(),
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate, TS)]
#[ts(export)]
pub struct CreateUserRequest {
    /// Username
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    /// Email address
    #[validate(regex(path = *EMAIL_RE))]
    pub email: String,
    /// Password
    #[validate(length(min = 8))]
    pub password: String,
    /// First name
    pub first_name: String,
    /// Last name
    pub last_name: String,
    /// Role UUIDs to assign to this user (optional)
    #[ts(type = "string[] | null")]
    pub role_uuids: Option<Vec<Uuid>>,
    /// Whether user is active
    pub is_active: Option<bool>,
    /// Super admin flag
    pub super_admin: Option<bool>,
}

/// Update user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate, TS)]
#[ts(export)]
pub struct UpdateUserRequest {
    /// Email address (optional)
    #[validate(regex(path = *EMAIL_RE))]
    pub email: Option<String>,
    /// Password (optional, only set if provided)
    #[validate(length(min = 8))]
    pub password: Option<String>,
    /// First name (optional)
    pub first_name: Option<String>,
    /// Last name (optional)
    pub last_name: Option<String>,
    /// Role UUIDs to assign to this user (optional)
    #[ts(type = "string[] | null")]
    pub role_uuids: Option<Vec<Uuid>>,
    /// Whether user is active (optional)
    pub is_active: Option<bool>,
    /// Super admin flag (optional)
    pub super_admin: Option<bool>,
}

#[cfg(test)]
mod tests {
    use r_data_core_core::validation::constraints;

    #[test]
    fn validation_constants_match_attributes() {
        assert_eq!(3, constraints::USERNAME_MIN_LENGTH);
        assert_eq!(50, constraints::USERNAME_MAX_LENGTH);
        assert_eq!(8, constraints::PASSWORD_MIN_LENGTH);
    }
}
