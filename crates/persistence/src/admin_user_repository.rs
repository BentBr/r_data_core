use crate::admin_user_repository_trait::{
    is_key_valid, AdminUserRepositoryTrait, ApiKeyRepositoryTrait,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use async_trait::async_trait;
use log::error;
use r_data_core_core::admin_user::{AdminUser, ApiKey};
use r_data_core_core::error::Result;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// Repository for API key operations
pub struct ApiKeyRepository {
    pool: Arc<Pool<Postgres>>,
}

impl ApiKeyRepository {
    /// Create a new repository instance
    #[must_use]
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

/// Repository for admin user operations
pub struct AdminUserRepository {
    pool: Arc<Pool<Postgres>>,
}

impl AdminUserRepository {
    /// Create a new repository instance
    #[must_use]
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminUserRepositoryTrait for AdminUserRepository {
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_users WHERE username = $1 OR email = $1",
        )
        .bind(username_or_email)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error finding user by username or email: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })
    }

    async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_users WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                error!("Error finding user by UUID: {:?}", e);
                r_data_core_core::error::Error::Database(e)
            })
    }

    async fn update_last_login(&self, uuid: &Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE admin_users SET last_login = NOW(), updated_at = NOW() WHERE uuid = $1",
        )
        .bind(uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error updating last login: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn create_admin_user<'a>(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        _role: Option<&'a str>,
        is_active: bool,
        creator_uuid: Uuid,
    ) -> Result<Uuid> {
        // Create UUID for the new user
        let user_uuid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Set default values
        let published = is_active; // Published if active
        let path = "/admin/users";

        // Hash the password using Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                r_data_core_core::error::Error::PasswordHash(format!(
                    "Failed to hash password: {}",
                    e
                ))
            })?
            .to_string();

        // For authenticated requests, use the authenticated user's UUID as creator
        // For unauthenticated requests or if creator_uuid is nil, use the new user's UUID
        let created_by = if !creator_uuid.is_nil() {
            creator_uuid
        } else {
            user_uuid // Self-reference for unauthenticated registrations
        };

        // Insert the new admin user
        sqlx::query!(
            "INSERT INTO admin_users (
                uuid, path, username, email, password_hash, first_name, last_name,
                is_active, created_at, updated_at, published, version, 
                created_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, 
                $8, $9, $9, $10, 1, 
                $11
            ) RETURNING uuid",
            user_uuid,
            path,
            username,
            email,
            &password_hash,
            first_name,
            last_name,
            is_active,
            now,
            published,
            created_by
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating admin user: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        // In the future, we'd store the role in a separate table or add it to this table
        // For now, we'll ignore the role parameter

        Ok(user_uuid)
    }

    async fn update_admin_user(&self, user: &AdminUser) -> Result<()> {
        sqlx::query!(
            "UPDATE admin_users SET 
                username = $1, 
                email = $2, 
                first_name = $3, 
                last_name = $4, 
                is_active = $5, 
                updated_at = $6,
                version = version + 1
            WHERE uuid = $7",
            user.username,
            user.email,
            user.first_name,
            user.last_name,
            user.is_active,
            OffsetDateTime::now_utc(),
            user.uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error updating admin user: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()> {
        // Soft delete by setting is_active to false
        sqlx::query!(
            "UPDATE admin_users SET is_active = false, updated_at = $1 WHERE uuid = $2",
            OffsetDateTime::now_utc(),
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error deleting admin user: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn list_admin_users(&self, limit: i64, offset: i64) -> Result<Vec<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error listing admin users: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })
    }
}

#[async_trait]
impl ApiKeyRepositoryTrait for ApiKeyRepository {
    /// Find an API key by its value, optimized for authentication
    async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>> {
        // Hash the provided API key
        let key_hash = ApiKey::hash_api_key(api_key)?;

        let api_key = sqlx::query_as::<_, ApiKey>(
            "
            SELECT *
            FROM api_keys
            WHERE key_hash = $1
            AND is_active = true
            AND (expires_at IS NULL OR expires_at > NOW())
            ",
        )
        .bind(key_hash)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error fetching API key: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        // Update last_used_at if the key was found
        if let Some(ref key) = api_key {
            self.update_last_used(key.uuid).await?;
            if is_key_valid(key) {
                return Ok(Some((key.clone(), key.user_uuid)));
            }
        }

        Ok(None)
    }

    /// Get a full API key by UUID (admin operations)
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>> {
        let api_key = sqlx::query_as::<_, ApiKey>(
            "
            SELECT *
            FROM api_keys
            WHERE uuid = $1
            ",
        )
        .bind(uuid)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error fetching API key by UUID: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(api_key)
    }

    /// Create a new API key
    async fn create(&self, key: &ApiKey) -> Result<Uuid> {
        let result = sqlx::query!(
            "
            INSERT INTO api_keys 
            (uuid, user_uuid, key_hash, name, description, is_active, created_at, expires_at, created_by, published)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING uuid
            ",
            key.uuid,
            key.user_uuid,
            key.key_hash,
            key.name,
            key.description,
            key.is_active,
            key.created_at,
            key.expires_at,
            key.created_by,
            key.published
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating API key: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(result.uuid)
    }

    /// List all API keys for a user
    async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiKey>> {
        let api_keys = sqlx::query_as::<_, ApiKey>(
            "
            SELECT *
            FROM api_keys
            WHERE user_uuid = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(user_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error listing API keys for user: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(api_keys)
    }

    /// Count API keys for a user
    async fn count_by_user(&self, user_uuid: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "
            SELECT COUNT(*)
            FROM api_keys
            WHERE user_uuid = $1
            ",
        )
        .bind(user_uuid)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error counting API keys for user: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(count)
    }

    /// Revoke an API key (set is_active to false)
    async fn revoke(&self, uuid: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET is_active = FALSE WHERE uuid = $1",
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error revoking API key: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Get an API key by its name for a specific user
    async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>> {
        let api_keys = sqlx::query_as::<_, ApiKey>(
            "
            SELECT *
            FROM api_keys 
            WHERE user_uuid = $1 AND name = $2
            ",
        )
        .bind(user_uuid)
        .bind(name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(api_keys)
    }

    /// Get an API key by its hash value
    async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>> {
        // Hash the provided API key
        let key_hash = ApiKey::hash_api_key(api_key)?;

        let api_keys = sqlx::query_as::<_, ApiKey>(
            "
            SELECT *
            FROM api_keys 
            WHERE key_hash = $1 AND is_active = true AND (expires_at IS NULL OR expires_at > now())
            ",
        )
        .bind(key_hash)
        .fetch_optional(&*self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(api_keys)
    }

    /// Create a new API key
    async fn create_new_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        // Validate input parameters
        if name.trim().is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "API key name cannot be empty".to_string(),
            ));
        }

        if expires_in_days < 0 {
            return Err(r_data_core_core::error::Error::Validation(
                "Expiration days cannot be negative".to_string(),
            ));
        }

        // Generate a secure random API key
        let key_value = ApiKey::generate_key();

        // Hash the key for storage
        let key_hash = ApiKey::hash_api_key(&key_value)?;

        // Create a new UUID for the API key
        let uuid = Uuid::now_v7();
        let created_at = OffsetDateTime::now_utc();
        let expires_at = if expires_in_days > 0 {
            Some(created_at + Duration::days(expires_in_days as i64))
        } else {
            None
        };

        let result = sqlx::query!(
            "
            INSERT INTO api_keys 
            (uuid, user_uuid, key_hash, name, description, is_active, created_at, expires_at, created_by, published)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING uuid
            ",
            uuid,
            created_by,  // Use the creator as the owner initially
            key_hash,
            name,
            description,
            true,        // Active by default
            created_at,
            expires_at,
            created_by,
            true         // Published by default
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating API key: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok((result.uuid, key_value))
    }

    /// Update an API key's last used timestamp
    async fn update_last_used(&self, uuid: Uuid) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        sqlx::query!(
            "UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2",
            now,
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error updating API key last_used_at: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Reassign an API key to a different user
    async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET user_uuid = $1 WHERE uuid = $2",
            new_user_uuid,
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error reassigning API key: {:?}", e);
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }
}
