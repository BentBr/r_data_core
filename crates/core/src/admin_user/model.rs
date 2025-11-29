#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::domain::AbstractRDataEntity;
use crate::error::{Error, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::engine::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sqlx::{
    decode::Decode,
    postgres::{PgRow, PgTypeInfo, PgValueRef},
    FromRow, Row, Type,
};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Admin user roles
///
/// Only `SuperAdmin` is predefined. All other roles are custom and defined
/// in permission schemes stored in the database.
/// Use `as_str()` to get the string representation for permission checking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    /// Super administrator with full access to all namespaces
    SuperAdmin,
    /// Custom role (name stored in the string, permissions defined in permission scheme)
    Custom(String),
}

impl UserRole {
    /// Get the string representation of the role for permission scheme lookups
    ///
    /// # Returns
    /// String representation matching permission scheme role names
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::SuperAdmin => "SuperAdmin",
            Self::Custom(name) => name,
        }
    }

    /// Convert a string to a `UserRole`
    ///
    /// # Arguments
    /// * `s` - String representation of the role
    ///
    /// # Returns
    /// `UserRole` enum variant
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "SuperAdmin" => Self::SuperAdmin,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl Type<sqlx::Postgres> for UserRole {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("VARCHAR")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for UserRole {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let value = <String as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(Self::from_str(&value))
    }
}

/// Admin user status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Inactive,
    Locked,
    PendingActivation,
}

impl Type<sqlx::Postgres> for UserStatus {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("VARCHAR")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for UserStatus {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let value = <String as Decode<sqlx::Postgres>>::decode(value)?;

        match value.as_str() {
            "Active" => Ok(Self::Active),
            "Inactive" => Ok(Self::Inactive),
            "Locked" => Ok(Self::Locked),
            "PendingActivation" => Ok(Self::PendingActivation),
            _ => Err("Invalid user status".into()),
        }
    }
}

/// Admin user representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminUser {
    /// Base entity properties
    #[serde(flatten)]
    pub base: AbstractRDataEntity,

    /// Username for login
    pub username: String,

    /// Email address
    pub email: String,

    /// Hashed password (not returned in API)
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// Full name
    pub full_name: String,

    /// User role
    pub role: UserRole,

    /// User account status
    pub status: UserStatus,

    /// Last login time
    pub last_login: Option<OffsetDateTime>,

    /// Failed login attempts
    pub failed_login_attempts: i32,

    /// Permission scheme ID
    pub permission_scheme_uuid: Option<Uuid>,

    pub uuid: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl<'r> FromRow<'r, PgRow> for AdminUser {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        // Get the basic fields that are guaranteed to be in the schema
        let uuid: Uuid = row.try_get("uuid")?;
        let username: String = row.try_get("username")?;
        let email: String = row.try_get("email")?;
        let password_hash: String = row.try_get("password_hash")?;
        let first_name: Option<String> = row.try_get("first_name")?;
        let last_name: Option<String> = row.try_get("last_name")?;
        let is_active: bool = row.try_get("is_active")?;
        let created_at: OffsetDateTime = row.try_get("created_at")?;
        let updated_at: OffsetDateTime = row.try_get("updated_at")?;

        // Build full name from first name and last name
        let full_name = match (&first_name, &last_name) {
            (Some(first), Some(last)) => format!("{first} {last}"),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => username.clone(),
        };

        // Get optional fields or use defaults for missing columns
        let last_login: Option<OffsetDateTime> = row.try_get("last_login").ok().flatten();

        // Use default values for fields that might not exist in the DB
        let role = UserRole::Custom("Default".to_string()); // Default role
        let status = UserStatus::Active; // Default status
        let failed_login_attempts = 0; // Default value
        let permission_scheme_uuid = None; // Default value
        let is_admin = false; // Default value

        Ok(Self {
            uuid,
            username,
            email,
            password_hash,
            full_name,
            role,
            status,
            last_login,
            failed_login_attempts,
            permission_scheme_uuid,
            first_name,
            last_name,
            is_active,
            is_admin,
            created_at,
            updated_at,
            base: AbstractRDataEntity::new("/admin/users".to_string()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub key_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_by: Uuid,
    pub published: bool,
}

impl AdminUser {
    /// Create a new admin user
    #[must_use]
    pub fn new(
        username: String,
        email: String,
        password_hash: String,
        full_name: String,
        role: UserRole,
        status: UserStatus,
        permission_scheme_uuid: Option<Uuid>,
        first_name: String,
        last_name: String,
        is_active: bool,
        is_admin: bool,
    ) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            base: AbstractRDataEntity::new("/admin/users".to_string()),
            username,
            email,
            password_hash,
            full_name,
            role,
            status,
            last_login: None,
            failed_login_attempts: 0,
            permission_scheme_uuid,
            uuid: Uuid::now_v7(),
            first_name: Some(first_name),
            last_name: Some(last_name),
            is_active,
            is_admin,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set password, hashing it with Argon2
    ///
    /// # Errors
    /// Returns an error if password is too short or hashing fails
    pub fn set_password(&mut self, password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(Error::Validation(
                "Password must be at least 8 characters long".to_string(),
            ));
        }

        // Use standardized Argon2id parameters:
        // - m=19456 (19 MB memory cost - good balance for modern systems)
        // - t=2 (2 iterations - recommended minimum)
        // - p=1 (1 parallelism - good for most systems)
        // Note: These parameters balance security and performance
        use argon2::Params;
        let params = Params::new(19456, 2, 1, None)
            .map_err(|e| Error::PasswordHash(format!("Invalid parameters: {e}")))?;

        // Hash the password using the Argon2id algorithm with standard params
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        self.password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::PasswordHash(format!("Failed to hash password: {e}")))?
            .to_string();

        Ok(())
    }

    /// Verify a password against the stored hash
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        // Parse the stored hash
        let Ok(parsed_hash) = PasswordHash::new(&self.password_hash) else {
            return false;
        };

        // Verify the password using Argon2id
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Check if the user has a specific permission using the permission scheme
    ///
    /// This method checks if the user's role has the specified permission type
    /// for the given namespace according to the permission scheme.
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to check against
    /// * `namespace` - Resource namespace
    /// * `permission_type` - Type of permission to check
    /// * `path` - Optional path constraint (for entities namespace)
    ///
    /// # Returns
    /// `true` if the user's role has the permission, `false` otherwise
    ///
    /// # Note
    /// SuperAdmin always returns `true` for all permissions.
    #[must_use]
    pub fn has_permission(
        &self,
        scheme: &crate::permissions::permission_scheme::PermissionScheme,
        namespace: &crate::permissions::permission_scheme::ResourceNamespace,
        permission_type: &crate::permissions::permission_scheme::PermissionType,
        path: Option<&str>,
    ) -> bool {
        // SuperAdmin has all permissions
        if matches!(self.role, UserRole::SuperAdmin) {
            return true;
        }

        scheme.has_permission(self.role.as_str(), namespace, permission_type, path)
    }

    /// Check if the user has a specific permission (simple string-based check for backward compatibility)
    ///
    /// This is a simplified check that returns true for SuperAdmin and Admin roles.
    /// For proper permission checking, use `has_permission` with a permission scheme.
    ///
    /// # Arguments
    /// * `_permission` - Permission string (not used, kept for backward compatibility)
    ///
    /// # Returns
    /// `true` if the user is `SuperAdmin` or Admin, `false` otherwise
    #[must_use]
    #[deprecated(note = "Use has_permission with PermissionScheme instead")]
    pub fn has_permission_simple(&self, _permission: &str) -> bool {
        matches!(self.role, UserRole::SuperAdmin)
    }

    /// Record a successful login
    pub fn record_login_success(&mut self) {
        self.last_login = Some(OffsetDateTime::now_utc());
        self.failed_login_attempts = 0;
    }

    /// Record a failed login attempt
    pub fn record_login_failure(&mut self) {
        self.failed_login_attempts += 1;
        if self.failed_login_attempts >= 5 {
            self.status = UserStatus::Locked;
        }
    }

    /// Check if the user can login
    #[must_use]
    pub fn can_login(&self) -> bool {
        self.is_active && self.status == UserStatus::Active
    }

    /// Get the user's full name
    #[must_use]
    pub fn full_name(&self) -> String {
        self.full_name.clone()
    }
}

impl ApiKey {
    /// Create a new API key
    #[must_use]
    pub fn new(
        user_uuid: Uuid,
        name: String,
        description: Option<String>,
        expires_at: Option<OffsetDateTime>,
        created_by: Uuid,
    ) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            uuid: Uuid::now_v7(),
            user_uuid,
            key_hash: String::new(), // Will be set separately
            name,
            description,
            is_active: true,
            created_at: now,
            expires_at,
            last_used_at: None,
            created_by,
            published: true,
        }
    }

    /// Generate a secure random API key
    #[must_use]
    pub fn generate_key() -> String {
        use base64::engine::general_purpose::STANDARD_NO_PAD;
        use rand::Rng;
        let mut rng = rand::rng();
        let token_bytes: [u8; 32] = rng.random();
        STANDARD_NO_PAD.encode(token_bytes)
    }

    /// Hash an API key for storage
    pub fn hash_api_key(api_key: &str) -> Result<String> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(api_key.as_bytes());
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }

    /// Check if an API key is valid (not expired)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if expires_at < OffsetDateTime::now_utc() {
                return false;
            }
        }

        true
    }

    /// Update the `last_used_at` timestamp
    pub async fn update_last_used(&mut self, pool: &sqlx::PgPool) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        self.last_used_at = Some(now);

        sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2")
            .bind(now)
            .bind(self.uuid)
            .execute(pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::permission_scheme::{
        AccessLevel, Permission, PermissionScheme, PermissionType,
    };

    #[test]
    fn test_user_role_as_str() {
        assert_eq!(UserRole::SuperAdmin.as_str(), "SuperAdmin");
        assert_eq!(UserRole::Custom("MyRole".to_string()).as_str(), "MyRole");
    }

    #[test]
    fn test_user_role_from_str() {
        assert_eq!(UserRole::from_str("SuperAdmin"), UserRole::SuperAdmin);
        assert_eq!(
            UserRole::from_str("MyRole"),
            UserRole::Custom("MyRole".to_string())
        );
    }

    #[test]
    fn test_admin_user_has_permission_superadmin() {
        let user = AdminUser::new(
            "admin".to_string(),
            "admin@test.com".to_string(),
            "hash".to_string(),
            "Admin User".to_string(),
            UserRole::SuperAdmin,
            UserStatus::Active,
            None,
            "Admin".to_string(),
            "User".to_string(),
            true,
            true,
        );

        let scheme = PermissionScheme::new("Test".to_string());

        // SuperAdmin should have all permissions
        assert!(user.has_permission(&scheme, "workflows", &PermissionType::Read, None));
        assert!(user.has_permission(
            &scheme,
            "entities",
            &PermissionType::Delete,
            Some("/any/path")
        ));
    }

    #[test]
    fn test_admin_user_has_permission_custom_role() {
        let scheme_uuid = Uuid::now_v7();
        let user = AdminUser::new(
            "user".to_string(),
            "user@test.com".to_string(),
            "hash".to_string(),
            "Test User".to_string(),
            UserRole::Custom("MyRole".to_string()),
            UserStatus::Active,
            Some(scheme_uuid),
            "Test".to_string(),
            "User".to_string(),
            true,
            false,
        );

        let mut scheme = PermissionScheme::new("Test".to_string());
        scheme
            .add_permission(
                "MyRole",
                Permission {
                    resource_type: ResourceNamespace::Workflows,
                    permission_type: PermissionType::Read,
                    access_level: AccessLevel::All,
                    resource_uuids: vec![],
                    constraints: None,
                },
            )
            .unwrap();

        // User has read permission
        assert!(user.has_permission(
            &scheme,
            &ResourceNamespace::Workflows,
            &PermissionType::Read,
            None
        ));

        // User does not have create permission
        assert!(!user.has_permission(
            &scheme,
            &ResourceNamespace::Workflows,
            &PermissionType::Create,
            None
        ));
    }
}
