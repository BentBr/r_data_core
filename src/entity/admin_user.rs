use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use utoipa::ToSchema;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        SaltString, PasswordHash, PasswordHasher, PasswordVerifier
    },
    Argon2
};
use sqlx::{FromRow, decode::Decode, postgres::{PgTypeInfo, PgValueRef}, Type};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use super::AbstractRDataEntity;
use crate::error::{Error, Result};

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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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
    pub permission_scheme_id: Option<i64>,
    
    pub id: Option<i64>,
    pub uuid: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Option<i64>,
    pub user_id: i64,
    pub api_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl AdminUser {
    /// Create a new admin user
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            base: AbstractRDataEntity::new("/admin/users".to_string()),
            username,
            email,
            password_hash,
            full_name: String::new(),
            role: UserRole::Viewer,
            status: UserStatus::PendingActivation,
            last_login: None,
            failed_login_attempts: 0,
            permission_scheme_id: None,
            id: None,
            uuid: Uuid::new_v4().to_string(),
            first_name: None,
            last_name: None,
            is_active: true,
            is_admin: false,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Set password, hashing it with Argon2
    pub fn set_password(&mut self, password: &str) -> Result<()> {
        if password.len() < 8 {
            return Err(Error::Validation("Password must be at least 8 characters long".to_string()));
        }

        // Hash the password using the Argon2 API
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        self.password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::PasswordHash(e.to_string()))?
            .to_string();
        
        Ok(())
    }
    
    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        let parsed_hash = match PasswordHash::new(&self.password_hash) {
            Ok(hash) => hash,
            Err(_) => return false,
        };
        
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
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
    pub fn new(user_id: i64, name: String, description: Option<String>, expires_at: Option<DateTime<Utc>>) -> Self {
        let now = Utc::now();
        let api_key = Self::generate_key();
        
        Self {
            id: None,
            user_id,
            api_key,
            name,
            description,
            is_active: true,
            created_at: now,
            expires_at,
            last_used_at: None,
        }
    }
    
    pub fn generate_key() -> String {
        // Generate a random API key (32 alphanumeric characters)
        let key: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
            
        format!("rdat-{}", key)
    }
    
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }
        
        if let Some(expires_at) = self.expires_at {
            if expires_at < Utc::now() {
                return false;
            }
        }
        
        true
    }
    
    pub async fn update_last_used(&mut self, pool: &sqlx::PgPool) -> Result<()> {
        let now = Utc::now();
        self.last_used_at = Some(now);
        
        if let Some(id) = self.id {
            sqlx::query!(
                "UPDATE api_keys SET last_used_at = $1 WHERE id = $2",
                now,
                id
            )
            .execute(pool)
            .await
            .map_err(|e| Error::Database(e))?;
        }
        
        Ok(())
    }
} 