#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::core::error::{Error, Result};
use crate::core::permissions::permission_scheme::PermissionScheme;
use crate::permission_scheme_repository_trait::PermissionSchemeRepositoryTrait;

/// Repository for permission scheme operations
pub struct PermissionSchemeRepository {
    pool: PgPool,
}

impl PermissionSchemeRepository {
    /// Create a new repository instance
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a permission scheme by UUID
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<PermissionScheme>> {
        let row = sqlx::query(
            r#"
            SELECT
                uuid, path, name, description,
                rules as "rules: serde_json::Value",
                created_at, updated_at, created_by, updated_by,
                published, version
            FROM permission_schemes
            WHERE uuid = $1
            "#,
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| {
            permission_scheme_from_row(&r).unwrap_or_else(|e| {
                log::error!("Failed to deserialize permission scheme: {}", e);
                // Return empty scheme on error
                PermissionScheme::new("Error".to_string())
            })
        }))
    }

    /// Get a permission scheme by name
    ///
    /// # Arguments
    /// * `name` - Scheme name
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_by_name(&self, name: &str) -> Result<Option<PermissionScheme>> {
        let row = sqlx::query(
            r#"
            SELECT
                uuid, path, name, description,
                rules as "rules: serde_json::Value",
                created_at, updated_at, created_by, updated_by,
                published, version
            FROM permission_schemes
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| {
            permission_scheme_from_row(&r).unwrap_or_else(|e| {
                log::error!("Failed to deserialize permission scheme: {}", e);
                PermissionScheme::new("Error".to_string())
            })
        }))
    }

    /// Create a new permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to create
    /// * `created_by` - UUID of user creating the scheme
    ///
    /// # Errors
    /// Returns an error if database insert fails
    pub async fn create(&self, scheme: &PermissionScheme, created_by: Uuid) -> Result<Uuid> {
        let uuid = scheme.base.uuid;
        let rules_json = serde_json::to_value(&scheme.role_permissions)
            .map_err(|e| Error::Unknown(format!("Failed to serialize role_permissions: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO permission_schemes (uuid, path, name, description, rules, created_by, published, version)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(uuid)
        .bind(&scheme.base.path)
        .bind(&scheme.name)
        .bind(&scheme.description)
        .bind(rules_json)
        .bind(created_by)
        .bind(scheme.base.published)
        .bind(scheme.base.version)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(uuid)
    }

    /// Update an existing permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to update
    /// * `updated_by` - UUID of user updating the scheme
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update(&self, scheme: &PermissionScheme, updated_by: Uuid) -> Result<()> {
        let rules_json = serde_json::to_value(&scheme.role_permissions)
            .map_err(|e| Error::Unknown(format!("Failed to serialize role_permissions: {e}")))?;

        sqlx::query(
            r#"
            UPDATE permission_schemes
            SET name = $2, description = $3, rules = $4, updated_by = $5, updated_at = NOW(), version = version + 1
            WHERE uuid = $1
            "#,
        )
        .bind(scheme.base.uuid)
        .bind(&scheme.name)
        .bind(&scheme.description)
        .bind(rules_json)
        .bind(updated_by)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Delete a permission scheme
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    pub async fn delete(&self, uuid: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM permission_schemes WHERE uuid = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// List all permission schemes with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of schemes to return
    /// * `offset` - Number of schemes to skip
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<PermissionScheme>> {
        let rows = sqlx::query(
            r#"
            SELECT
                uuid, path, name, description,
                rules as "rules: serde_json::Value",
                created_at, updated_at, created_by, updated_by,
                published, version
            FROM permission_schemes
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut schemes = Vec::new();
        for row in rows {
            match permission_scheme_from_row(&row) {
                Ok(scheme) => schemes.push(scheme),
                Err(e) => {
                    log::error!("Failed to deserialize permission scheme: {}", e);
                    // Continue with other schemes
                }
            }
        }

        Ok(schemes)
    }

    /// Count all permission schemes
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count_all(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM permission_schemes")
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }
}

#[async_trait]
impl PermissionSchemeRepositoryTrait for PermissionSchemeRepository {
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<PermissionScheme>> {
        Self::get_by_uuid(self, uuid).await
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<PermissionScheme>> {
        Self::get_by_name(self, name).await
    }

    async fn create(&self, scheme: &PermissionScheme, created_by: Uuid) -> Result<Uuid> {
        Self::create(self, scheme, created_by).await
    }

    async fn update(&self, scheme: &PermissionScheme, updated_by: Uuid) -> Result<()> {
        Self::update(self, scheme, updated_by).await
    }

    async fn delete(&self, uuid: Uuid) -> Result<()> {
        Self::delete(self, uuid).await
    }
}

/// Helper function to deserialize PermissionScheme from database row
fn permission_scheme_from_row(row: &PgRow) -> std::result::Result<PermissionScheme, sqlx::Error> {
    use crate::core::domain::AbstractRDataEntity;
    use std::collections::HashMap;

    let uuid: Uuid = row.try_get("uuid")?;
    let path: String = row.try_get("path")?;
    let name: String = row.try_get("name")?;
    let description: Option<String> = row.try_get("description").ok();
    let created_at: time::OffsetDateTime = row.try_get("created_at")?;
    let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
    let created_by: Uuid = row.try_get("created_by")?;
    let updated_by: Option<Uuid> = row.try_get("updated_by").ok();
    let published: bool = row.try_get("published").unwrap_or(false);
    let version: i32 = row.try_get("version").unwrap_or(1);

    let base = AbstractRDataEntity {
        uuid,
        path,
        created_at,
        updated_at,
        created_by,
        updated_by,
        published,
        version,
        custom_fields: HashMap::new(),
    };

    // Deserialize rules JSONB to role_permissions
    let rules_json: serde_json::Value = row.try_get("rules")?;
    let role_permissions: HashMap<
        String,
        Vec<crate::core::permissions::permission_scheme::Permission>,
    > = serde_json::from_value(rules_json).map_err(|e| {
        sqlx::Error::Decode(format!("Failed to deserialize role_permissions: {e}").into())
    })?;

    Ok(PermissionScheme {
        base,
        name,
        description,
        is_system: false, // System schemes are determined by application logic
        role_permissions,
    })
}
