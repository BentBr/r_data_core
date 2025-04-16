use super::AbstractRDataEntity;
use crate::error::{Error, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{
    decode::Decode,
    postgres::{PgRow, PgTypeInfo, PgValueRef},
    FromRow, Row, Type,
};
use time;
use utoipa::ToSchema;
use uuid::timestamp;
use uuid::{ContextV7, Uuid};

/// Admin user roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    SuperAdmin,
    Admin,
    Editor,
    Viewer,
    Custom(String),
}

impl Type<sqlx::Postgres> for UserRole {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("VARCHAR")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for UserRole {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let value = <String as Decode<sqlx::Postgres>>::decode(value)?;

        match value.as_str() {
            "SuperAdmin" => Ok(UserRole::SuperAdmin),
            "Admin" => Ok(UserRole::Admin),
            "Editor" => Ok(UserRole::Editor),
            "Viewer" => Ok(UserRole::Viewer),
            other => Ok(UserRole::Custom(other.to_string())),
        }
    }
}

/// Admin user status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            "Active" => Ok(UserStatus::Active),
            "Inactive" => Ok(UserStatus::Inactive),
            "Locked" => Ok(UserStatus::Locked),
            "PendingActivation" => Ok(UserStatus::PendingActivation),
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
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,

    /// Failed login attempts
    pub failed_login_attempts: i32,

    /// Permission scheme ID
    pub permission_scheme_uuid: Option<Uuid>,

    pub uuid: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        // Build full name from first name and last name
        let full_name = match (&first_name, &last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => username.clone(),
        };

        // Get optional fields or use defaults for missing columns
        let last_login: Option<DateTime<Utc>> = row.try_get("last_login").unwrap_or(None);

        // Use default values for fields that might not exist in the DB
        let role = UserRole::Admin; // Default role
        let status = UserStatus::Active; // Default status
        let failed_login_attempts = 0; // Default value
        let permission_scheme_uuid = None; // Default value
        let is_admin = false; // Default value

        Ok(AdminUser {
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
    pub uuid: Option<Uuid>,
    pub user_uuid: Uuid,
    pub api_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
    pub last_used_at: Option<time::OffsetDateTime>,
}

impl AdminUser {
    /// Create a new admin user
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
        let now = Utc::now();
        let context = ContextV7::new();
        let ts = timestamp::Timestamp::now(&context);
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
            uuid: Uuid::new_v7(ts),
            first_name: Some(first_name),
            last_name: Some(last_name),
            is_active,
            is_admin,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set password, hashing it with Argon2
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
            .map_err(|e| Error::PasswordHash(format!("Invalid parameters: {}", e)))?;

        // Hash the password using the Argon2id algorithm with standard params
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        self.password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::PasswordHash(e.to_string()))?
            .to_string();

        log::debug!("Created password hash: {}", self.password_hash);
        Ok(())
    }

    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        log::debug!("Verifying password against hash: {}", self.password_hash);

        // Try to parse the hash
        let parsed_hash = match PasswordHash::new(&self.password_hash) {
            Ok(hash) => hash,
            Err(e) => {
                log::error!("Failed to parse password hash: {:?}", e);
                return false;
            }
        };

        log::debug!("Hash parsed successfully");

        // Check if this is a migration hash (very low memory parameter)
        if self.password_hash.contains("m=16,t=2,p=1") {
            log::debug!("Detected legacy low-memory hash format");
            // Create a custom argon2 configuration with low memory parameters
            use argon2::Params;
            let legacy_params = Params::new(16, 2, 1, None).unwrap();
            let legacy_argon2 = Argon2::new(
                argon2::Algorithm::Argon2id,
                argon2::Version::V0x13,
                legacy_params,
            );

            return legacy_argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok();
        }

        // Standard verification with default (or specified) parameters
        let result = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        log::debug!("Password verification result: {}", result);
        result
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, _permission: &str) -> bool {
        // SuperAdmin always has all permissions
        if let UserRole::SuperAdmin = self.role {
            return true;
        }

        // TODO: Implement permission checking with PermissionScheme
        // This is a placeholder for the actual implementation
        false
    }

    /// Record a successful login
    pub fn record_login_success(&mut self) {
        self.last_login = Some(chrono::Utc::now());
        self.failed_login_attempts = 0;
    }

    /// Record a failed login attempt
    pub fn record_login_failure(&mut self) {
        self.failed_login_attempts += 1;

        // Automatically lock account after too many failed attempts
        if self.failed_login_attempts >= 5 {
            self.status = UserStatus::Locked;
        }
    }

    /// Check if the user account is active and can log in
    pub fn can_login(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    pub fn full_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.username.clone(),
        }
    }
}

impl ApiKey {
    pub fn new(
        user_uuid: Uuid,
        name: String,
        description: Option<String>,
        expires_at: Option<time::OffsetDateTime>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            uuid: None,
            user_uuid,
            api_key: Self::generate_key(),
            name,
            description,
            is_active: true,
            created_at: now,
            expires_at,
            last_used_at: None,
        }
    }

    /// Generate a random API key
    pub fn generate_key() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::rng();
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Check if the API key is valid (not expired and active)
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if expires_at < time::OffsetDateTime::now_utc() {
                return false;
            }
        }

        true
    }

    /// Update the last used timestamp
    pub async fn update_last_used(&mut self, pool: &sqlx::PgPool) -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
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
