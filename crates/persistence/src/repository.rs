use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};
use std::marker::PhantomData;
use uuid::Uuid;

use r_data_core_core::error::{Error, Result};
use r_data_core_core::versioning::VersionedData;

/// Extension trait for ``PgPool`` to add `repository()` function
pub trait PgPoolExtension {
    /// Get a repository for a specific entity type
    fn repository<T>(&self) -> EntityRepository<T>
    where
        T: Send
            + Sync
            + Unpin
            + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + serde::Serialize
            + DeserializeOwned
            + 'static;

    /// Get a repository for a specific entity type with a specified table name
    fn repository_with_table<T>(&self, table_name: &str) -> EntityRepository<T>
    where
        T: Send
            + Sync
            + Unpin
            + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + serde::Serialize
            + DeserializeOwned
            + 'static;
}

impl PgPoolExtension for PgPool {
    fn repository<T>(&self) -> EntityRepository<T>
    where
        T: Send
            + Sync
            + Unpin
            + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + serde::Serialize
            + DeserializeOwned
            + 'static,
    {
        // Use a default table name based on T (placeholder)
        EntityRepository::new(self.clone(), "default_table")
    }

    fn repository_with_table<T>(&self, table_name: &str) -> EntityRepository<T>
    where
        T: Send
            + Sync
            + Unpin
            + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + serde::Serialize
            + DeserializeOwned
            + 'static,
    {
        EntityRepository::new(self.clone(), table_name)
    }
}

/// Base repository for entity database operations
pub struct EntityRepository<T> {
    pool: PgPool,
    table_name: String,
    _phantom: PhantomData<T>,
}

impl<T> EntityRepository<T>
where
    T: Send
        + Sync
        + Unpin
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Serialize
        + DeserializeOwned
        + 'static,
{
    /// Create a new repository with a database connection pool
    #[must_use]
    pub fn new(pool: PgPool, table_name: &str) -> Self {
        Self {
            pool,
            table_name: table_name.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Get the database connection pool
    #[must_use]
    pub const fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get an entity by UUID
    ///
    /// # Errors
    /// Returns an error if the entity is not found or database query fails
    pub async fn get_by_uuid(&self, uuid: &Uuid) -> Result<T> {
        let query = format!("SELECT * FROM {} WHERE uuid = $1", self.table_name);

        sqlx::query_as::<_, T>(&query)
            .bind(uuid)
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)
    }

    /// List entities with filtering and pagination
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list(
        &self,
        filter: Option<&str>,
        sort: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<T>> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new(format!("SELECT * FROM {}", self.table_name));

        if let Some(filter_str) = filter {
            query_builder.push(" WHERE ");
            query_builder.push(filter_str);
        }

        if let Some(sort_str) = sort {
            query_builder.push(" ORDER BY ");
            query_builder.push(sort_str);
        }

        if let Some(limit_val) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit_val);
        }

        if let Some(offset_val) = offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset_val);
        }

        let query = query_builder.build_query_as::<T>();

        query.fetch_all(&self.pool).await.map_err(Error::Database)
    }

    /// Count entities in the table, optionally filtered
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count(&self, filter: Option<&str>) -> Result<i64> {
        let query_str = filter.map_or_else(
            || format!("SELECT COUNT(*) AS count FROM {}", self.table_name),
            |filter_clause| {
                format!(
                    "SELECT COUNT(*) AS count FROM {} WHERE {}",
                    self.table_name, filter_clause
                )
            },
        );

        let query = sqlx::query(&query_str);

        let row = query.fetch_one(&self.pool).await.map_err(Error::Database)?;

        let count: i64 = row.try_get("count").map_err(Error::Database)?;

        Ok(count)
    }

    /// Save a new entity
    ///
    /// # Errors
    /// Returns an error if serialization or database operation fails
    pub async fn create(&self, entity: &T) -> Result<Uuid> {
        // Serialize entity to JSON for insertion
        let json = serde_json::to_value(entity).map_err(Error::Serialization)?;

        // Extract fields we need for all entities
        let uuid = json
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing uuid".to_string()))?;
        let path = json
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing path".to_string()))?;

        // Basic insert query with common fields
        let query = format!(
            "INSERT INTO {} (uuid, path, created_at, updated_at, created_by, updated_by, published, version, custom_fields)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING uuid",
            self.table_name
        );

        // Get values for common fields
        let uuid = Uuid::parse_str(uuid).map_err(|_| Error::Entity("Invalid UUID".to_string()))?;
        let created_at = json
            .get("created_at")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing created_at".to_string()))?;
        let updated_at = json
            .get("updated_at")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing updated_at".to_string()))?;
        let created_by = json
            .get("created_by")
            .and_then(|v| v.as_str())
            .map(|v| Uuid::parse_str(v).ok());
        let updated_by = json
            .get("updated_by")
            .and_then(|v| v.as_str())
            .map(|v| Uuid::parse_str(v).ok());
        let published = json
            .get("published")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let version = json
            .get("version")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(1);

        // Get custom fields
        let custom_fields = json
            .get("custom_fields")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // Execute query
        let uuid: (Uuid,) = sqlx::query_as(&query)
            .bind(uuid)
            .bind(path)
            .bind(created_at)
            .bind(updated_at)
            .bind(created_by)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .bind(custom_fields)
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(uuid.0)
    }

    /// Update an existing entity
    ///
    /// # Errors
    /// Returns an error if entity not found, serialization fails, or database operation fails
    ///
    /// # Note
    /// Versioning is handled by specific repositories (e.g., `DynamicEntityRepository` uses
    /// `VersionRepository::snapshot_pre_update_tx`). This generic repository does not handle versioning.
    pub async fn update(&self, uuid: &Uuid, entity: &T) -> Result<()> {
        // Serialize entity to JSON for update
        let json = serde_json::to_value(entity).map_err(Error::Serialization)?;

        // Extract fields we need for all entities
        let path = json
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing path".to_string()))?;
        let updated_at = json
            .get("updated_at")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Entity("Missing updated_at".to_string()))?;
        let updated_by = json
            .get("updated_by")
            .and_then(|v| v.as_str())
            .map(|v| Uuid::parse_str(v).ok());
        let published = json
            .get("published")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let version = json
            .get("version")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(1);

        // Get custom fields
        let custom_fields = json
            .get("custom_fields")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // Basic update query with common fields
        let query = format!(
            "UPDATE {} SET path = $1, updated_at = $2, updated_by = $3, published = $4, version = $5, custom_fields = $6 WHERE uuid = $7",
            self.table_name
        );

        // Execute query
        sqlx::query(&query)
            .bind(path)
            .bind(updated_at)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .bind(custom_fields)
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// Delete an entity
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn delete(&self, uuid: &Uuid) -> Result<()> {
        let query = format!("DELETE FROM {} WHERE uuid = $1", self.table_name);

        sqlx::query(&query)
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// Get a specific version of an entity
    ///
    /// # Errors
    /// Returns an error if version not found or database query fails
    pub async fn get_version(&self, uuid: &Uuid, version: i32) -> Result<VersionedData> {
        sqlx::query_as::<_, VersionedData>(
            "SELECT * FROM entities_versions WHERE entity_uuid = $1 AND version_number = $2",
        )
        .bind(uuid)
        .bind(version)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)
    }

    /// List versions of an entity
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_versions(&self, uuid: &Uuid) -> Result<Vec<VersionedData>> {
        sqlx::query_as::<_, VersionedData>(
            "SELECT * FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number DESC",
        )
        .bind(uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)
    }
}
