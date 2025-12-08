#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin_user_repository_trait::{is_key_valid, ApiKeyRepositoryTrait};
use async_trait::async_trait;
use log::{debug, error};
use r_data_core_core::admin_user::ApiKey;
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
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to Arc parameter
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }

    /// Get all roles assigned to an API key
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_api_key_roles(&self, api_key_uuid: Uuid) -> Result<Vec<Uuid>> {
        let roles = sqlx::query_scalar::<_, Uuid>(
            "SELECT role_uuid FROM api_key_roles WHERE api_key_uuid = $1",
        )
        .bind(api_key_uuid)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error getting API key roles: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(roles)
    }

    /// Get all API keys that have a specific role assigned
    ///
    /// # Arguments
    /// * `role_uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_api_keys_by_role(&self, role_uuid: Uuid) -> Result<Vec<Uuid>> {
        let api_keys = sqlx::query_scalar::<_, Uuid>(
            "SELECT api_key_uuid FROM api_key_roles WHERE role_uuid = $1",
        )
        .bind(role_uuid)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error getting API keys by role: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(api_keys)
    }

    /// Assign a role to an API key
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn assign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO api_key_roles (api_key_uuid, role_uuid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(api_key_uuid)
        .bind(role_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error assigning role to API key: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Unassign a role from an API key
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn unassign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM api_key_roles WHERE api_key_uuid = $1 AND role_uuid = $2")
            .bind(api_key_uuid)
            .bind(role_uuid)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                error!("Error unassigning role from API key: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;

        Ok(())
    }

    /// Update all roles for an API key (replace existing assignments)
    ///
    /// # Errors
    /// Returns an error if the database transaction fails
    pub async fn update_api_key_roles(
        &self,
        api_key_uuid: Uuid,
        role_uuids: &[Uuid],
    ) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            error!("Error starting transaction: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        // Delete all existing assignments
        sqlx::query("DELETE FROM api_key_roles WHERE api_key_uuid = $1")
            .bind(api_key_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Error deleting existing roles: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;

        // Insert new assignments
        for role_uuid in role_uuids {
            sqlx::query("INSERT INTO api_key_roles (api_key_uuid, role_uuid) VALUES ($1, $2)")
                .bind(api_key_uuid)
                .bind(role_uuid)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!("Error assigning role: {e:?}");
                    r_data_core_core::error::Error::Database(e)
                })?;
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            error!("Error committing transaction: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
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
            error!("Error fetching API key: {e:?}");
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
            error!("Error fetching API key by UUID: {e:?}");
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
            // Log foreign key constraint violations at debug level since they're often expected in tests
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.code().as_deref() == Some("23503") {
                    debug!("Foreign key constraint violation creating API key (expected in some tests): {e:?}");
                } else {
                    error!("Error creating API key: {e:?}");
                }
            } else {
                error!("Error creating API key: {e:?}");
            }
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(result.uuid)
    }

    /// List all API keys for a user
    async fn list_by_user(
        &self,
        user_uuid: Uuid,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<ApiKey>> {
        // Build ORDER BY clause - field is already validated and sanitized by route handler
        let order_by = sort_by.map_or_else(
            || "\"created_at\" DESC".to_string(),
            |field| {
                let quoted_field = format!("\"{}\"", field.replace('"', "\"\""));
                let order = sort_order
                    .as_ref()
                    .map(|o| o.to_uppercase())
                    .filter(|o| o == "ASC" || o == "DESC")
                    .unwrap_or_else(|| "ASC".to_string());

                // Handle NULL values for last_used_at and expires_at
                if field == "last_used_at" || field == "expires_at" {
                    format!("{quoted_field} {order} NULLS LAST")
                } else {
                    format!("{quoted_field} {order}")
                }
            },
        );

        // Build query with or without LIMIT
        let query = if limit == -1 {
            format!("SELECT * FROM api_keys WHERE user_uuid = $1 ORDER BY {order_by} OFFSET $2")
        } else {
            format!("SELECT * FROM api_keys WHERE user_uuid = $1 ORDER BY {order_by} LIMIT $2 OFFSET $3")
        };

        let mut query_builder = sqlx::query_as::<_, ApiKey>(&query);
        query_builder = query_builder.bind(user_uuid);
        if limit == -1 {
            query_builder = query_builder.bind(offset);
        } else {
            query_builder = query_builder.bind(limit).bind(offset);
        }

        query_builder.fetch_all(&*self.pool).await.map_err(|e| {
            error!("Error listing API keys for user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })
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
            error!("Error counting API keys for user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(count)
    }

    /// Revoke an API key (set `is_active` to false)
    async fn revoke(&self, uuid: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE api_keys SET is_active = FALSE WHERE uuid = $1",
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error revoking API key: {e:?}");
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
            Some(created_at + Duration::days(i64::from(expires_in_days)))
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
            // Log foreign key constraint violations at debug level since they're often expected in tests
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.code().as_deref() == Some("23503") {
                    debug!("Foreign key constraint violation creating API key (expected in some tests): {e:?}");
                } else {
                    error!("Error creating API key: {e:?}");
                }
            } else {
                error!("Error creating API key: {e:?}");
            }
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
            error!("Error updating API key last_used_at: {e:?}");
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
            error!("Error reassigning API key: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Get all roles assigned to an API key
    async fn get_api_key_roles(&self, api_key_uuid: Uuid) -> Result<Vec<Uuid>> {
        Self::get_api_key_roles(self, api_key_uuid).await
    }

    /// Assign a role to an API key
    async fn assign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        Self::assign_role(self, api_key_uuid, role_uuid).await
    }

    /// Unassign a role from an API key
    async fn unassign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        Self::unassign_role(self, api_key_uuid, role_uuid).await
    }

    /// Update all roles for an API key (replace existing assignments)
    async fn update_api_key_roles(&self, api_key_uuid: Uuid, role_uuids: &[Uuid]) -> Result<()> {
        Self::update_api_key_roles(self, api_key_uuid, role_uuids).await
    }
}
