use crate::entity::admin_user::ApiKey;
use crate::error::{Error, Result};
use log::{debug, error};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::{timestamp, ContextV7, Uuid};

/// Repository for API key operations
pub struct ApiKeyRepository {
    pool: Arc<Pool<Postgres>>,
}

impl ApiKeyRepository {
    /// Create a new repository instance
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }

    /// Find an API key by its value, optimized for authentication
    /// Only fetches the essential fields needed for validation
    pub async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<ApiKey>> {
        // Hash the provided API key
        let key_hash = ApiKey::hash_api_key(api_key)?;

        let api_key = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT *
            FROM api_keys
            WHERE key_hash = $1
            AND is_active = true
            AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error fetching API key: {:?}", e);
            Error::Database(e)
        })?;

        // Update last_used_at if the key was found
        if let Some(api_key) = &api_key {
            self.update_last_used(api_key.uuid).await?;
        }

        Ok(api_key)
    }

    /// Get a full API key by UUID (admin operations)
    pub async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>> {
        let api_key = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT *
            FROM api_keys
            WHERE uuid = $1
            "#,
        )
        .bind(uuid)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error fetching API key by UUID: {:?}", e);
            Error::Database(e)
        })?;

        Ok(api_key)
    }

    /// Create a new API key
    pub async fn create(&self, key: &ApiKey) -> Result<Uuid> {
        let result = sqlx::query!(
            r#"
            INSERT INTO api_keys 
            (uuid, user_uuid, key_hash, name, description, is_active, created_at, expires_at, created_by, published)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING uuid
            "#,
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
            Error::Database(e)
        })?;

        Ok(result.uuid)
    }

    /// Update an API key's last used timestamp
    async fn update_api_key_usage(key_uuid: Uuid, pool: &Pool<Postgres>) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        sqlx::query!(
            "UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2",
            now,
            key_uuid
        )
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Error updating API key last_used_at: {:?}", e);
            Error::Database(e)
        })?;

        Ok(())
    }

    /// Check if an API key is valid (not expired)
    fn is_key_valid(&self, key: &ApiKey) -> bool {
        if !key.is_active {
            return false;
        }

        if let Some(expires_at) = key.expires_at {
            if expires_at < OffsetDateTime::now_utc() {
                return false;
            }
        }

        true
    }

    /// List all API keys for a user
    pub async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiKey>> {
        let api_keys = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT *
            FROM api_keys
            WHERE user_uuid = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error listing API keys for user: {:?}", e);
            Error::Database(e)
        })?;

        Ok(api_keys)
    }

    /// Revoke an API key (set is_active to false)
    pub async fn revoke(&self, uuid: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET is_active = FALSE WHERE uuid = $1",
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error revoking API key: {:?}", e);
            Error::Database(e)
        })?;

        Ok(())
    }

    /// Get an API key by its name for a specific user
    pub async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>> {
        let api_keys = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT *
            FROM api_keys 
            WHERE user_uuid = $1 AND name = $2
            "#,
        )
        .bind(user_uuid)
        .bind(name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(api_keys)
    }

    /// Get an API key by its hash value
    pub async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>> {
        // Hash the provided API key
        let key_hash = ApiKey::hash_api_key(api_key)?;

        let api_keys = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT *
            FROM api_keys 
            WHERE key_hash = $1 AND is_active = true AND (expires_at IS NULL OR expires_at > now())
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&*self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(api_keys)
    }

    /// Create a new API key
    pub async fn create_new_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        // Generate a secure random API key
        let key_value = ApiKey::generate_key();

        // Hash the key for storage
        let key_hash = ApiKey::hash_api_key(&key_value)?;

        // Create a new UUID for the API key
        let context = ContextV7::new();
        let ts = timestamp::Timestamp::now(&context);
        let uuid = Uuid::new_v7(ts);
        let created_at = OffsetDateTime::now_utc();
        let expires_at = if expires_in_days > 0 {
            Some(created_at + Duration::days(expires_in_days as i64))
        } else {
            None
        };

        let result = sqlx::query!(
            r#"
            INSERT INTO api_keys 
            (uuid, user_uuid, key_hash, name, description, is_active, created_at, expires_at, created_by, published)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING uuid
            "#,
            uuid,
            created_by,
            key_hash,
            Some(name.to_string()),
            Some(description.to_string()),
            true,
            created_at,
            expires_at,
            created_by,
            true
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating API key: {:?}", e);
            Error::Database(e)
        })?;

        Ok((result.uuid, key_value))
    }

    /// Update the last_used_at timestamp for an API key
    pub async fn update_last_used(&self, uuid: Uuid) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        sqlx::query!(
            "UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2",
            now,
            uuid
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    /// Reassign an API key to a different user
    pub async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET user_uuid = $1 WHERE uuid = $2",
            new_user_uuid,
            uuid
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }
}
