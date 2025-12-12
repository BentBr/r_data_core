#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

use crate::core::error::{Error, Result};
use crate::core::permissions::role::Role;
use crate::role_repository_trait::RoleRepositoryTrait;

/// Repository for role operations
pub struct RoleRepository {
    pool: PgPool,
}

impl RoleRepository {
    /// Create a new repository instance
    ///
    /// # Arguments
    /// * `pool` - `PostgreSQL` connection ``PgPool``
    ///
    /// # Errors
    /// This function does not return errors
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a role by UUID
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Role>> {
        let row = sqlx::query(
            r#"
            SELECT
                uuid, path, name, description,
                permissions as "permissions: serde_json::Value",
                is_system, super_admin,
                created_at, updated_at, created_by, updated_by,
                published, version
            FROM roles
            WHERE uuid = $1
            "#,
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        row.map_or_else(
            || Ok(None),
            |r| {
                role_from_row(&r).map(Some).map_err(|e| {
                    log::error!("Failed to deserialize role: {e}");
                    Error::Unknown(format!("Failed to deserialize role: {e}"))
                })
            },
        )
    }

    /// Get a role by name
    ///
    /// # Arguments
    /// * `name` - Role name
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Role>> {
        let row = sqlx::query(
            r#"
            SELECT
                uuid, path, name, description,
                permissions as "permissions: serde_json::Value",
                is_system, super_admin,
                created_at, updated_at, created_by, updated_by,
                published, version
            FROM roles
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        row.map_or_else(
            || Ok(None),
            |r| {
                role_from_row(&r).map(Some).map_err(|e| {
                    log::error!("Failed to deserialize role: {e}");
                    Error::Unknown(format!("Failed to deserialize role: {e}"))
                })
            },
        )
    }

    /// Create a new role
    ///
    /// # Arguments
    /// * `role` - Role to create
    /// * `created_by` - UUID of user creating the role
    ///
    /// # Errors
    /// Returns an error if database insert fails
    pub async fn create(&self, role: &Role, created_by: Uuid) -> Result<Uuid> {
        let uuid = role.base.uuid;
        let permissions_json = serde_json::to_value(&role.permissions)
            .map_err(|e| Error::Unknown(format!("Failed to serialize permissions: {e}")))?;

        sqlx::query(
            "
            INSERT INTO roles (uuid, path, name, description, permissions, is_system, super_admin, created_by, published, version)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ",
        )
        .bind(uuid)
        .bind(&role.base.path)
        .bind(&role.name)
        .bind(&role.description)
        .bind(permissions_json)
        .bind(role.is_system)
        .bind(role.super_admin)
        .bind(created_by)
        .bind(role.base.published)
        .bind(role.base.version)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(uuid)
    }

    /// Update an existing role
    ///
    /// # Arguments
    /// * `role` - Role to update
    /// * `updated_by` - UUID of user updating the role
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update(&self, role: &Role, updated_by: Uuid) -> Result<()> {
        let permissions_json = serde_json::to_value(&role.permissions)
            .map_err(|e| Error::Unknown(format!("Failed to serialize permissions: {e}")))?;

        sqlx::query(
            "
            UPDATE roles
            SET name = $2, description = $3, permissions = $4, super_admin = $5, updated_by = $6, updated_at = NOW(), version = version + 1
            WHERE uuid = $1 AND is_system = FALSE
            ",
        )
        .bind(role.base.uuid)
        .bind(&role.name)
        .bind(&role.description)
        .bind(permissions_json)
        .bind(role.super_admin)
        .bind(updated_by)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Delete a role
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    pub async fn delete(&self, uuid: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM roles WHERE uuid = $1 AND is_system = FALSE")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// List all roles with pagination and sorting
    ///
    /// # Arguments
    /// * `limit` - Maximum number of roles to return (-1 for unlimited)
    /// * `offset` - Number of roles to skip
    /// * `sort_by` - Optional field to sort by
    /// * `sort_order` - Sort order (ASC or DESC), defaults to ASC
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_all(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Role>> {
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
                format!("{quoted_field} {order}")
            },
        );

        // Build query with or without LIMIT
        let query = if limit == i64::MAX {
            format!(
                r#"
                SELECT
                    uuid, path, name, description,
                    permissions as "permissions: serde_json::Value",
                    is_system, super_admin,
                    created_at, updated_at, created_by, updated_by,
                    published, version
                FROM roles
                ORDER BY {order_by} OFFSET $1
                "#
            )
        } else {
            format!(
                r#"
                SELECT
                    uuid, path, name, description,
                    permissions as "permissions: serde_json::Value",
                    is_system, super_admin,
                    created_at, updated_at, created_by, updated_by,
                    published, version
                FROM roles
                ORDER BY {order_by} LIMIT $1 OFFSET $2
                "#
            )
        };

        let mut query_builder = sqlx::query(&query);
        if limit == i64::MAX {
            query_builder = query_builder.bind(offset);
        } else {
            query_builder = query_builder.bind(limit).bind(offset);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let mut roles = Vec::new();
        for row in rows {
            match role_from_row(&row) {
                Ok(role) => roles.push(role),
                Err(e) => {
                    log::error!("Failed to deserialize role: {e}");
                    // Continue with other roles
                }
            }
        }

        Ok(roles)
    }

    /// Count all roles
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count_all(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM roles")
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }
}

#[async_trait]
impl RoleRepositoryTrait for RoleRepository {
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Role>> {
        Self::get_by_uuid(self, uuid).await
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Role>> {
        Self::get_by_name(self, name).await
    }

    async fn create(&self, role: &Role, created_by: Uuid) -> Result<Uuid> {
        Self::create(self, role, created_by).await
    }

    async fn update(&self, role: &Role, updated_by: Uuid) -> Result<()> {
        Self::update(self, role, updated_by).await
    }

    async fn delete(&self, uuid: Uuid) -> Result<()> {
        Self::delete(self, uuid).await
    }

    async fn list_all(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Role>> {
        Self::list_all(self, limit, offset, sort_by, sort_order).await
    }

    async fn count_all(&self) -> Result<i64> {
        Self::count_all(self).await
    }
}

/// Helper function to deserialize `Role` from database row
///
/// # Panics
/// May panic if database row data is invalid
fn role_from_row(row: &PgRow) -> std::result::Result<Role, sqlx::Error> {
    use crate::core::domain::AbstractRDataEntity;
    use std::collections::HashMap;

    let uuid: Uuid = row.try_get("uuid")?;
    let path: String = row.try_get("path")?;
    let name: String = row.try_get("name")?;
    let description: Option<String> = row.try_get("description").ok();
    let is_system: bool = row.try_get("is_system").unwrap_or(false);
    let super_admin: bool = row.try_get("super_admin").unwrap_or(false);
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

    // Deserialize permissions JSONB to Vec<Permission>
    // Try accessing with the type annotation name first, fall back to "permissions"
    let permissions_json: serde_json::Value = row
        .try_get("permissions: serde_json::Value")
        .or_else(|_| row.try_get("permissions"))?;
    let permissions: Vec<crate::core::permissions::role::Permission> =
        serde_json::from_value(permissions_json).map_err(|e| {
            sqlx::Error::Decode(format!("Failed to deserialize permissions: {e}").into())
        })?;

    Ok(Role {
        base,
        name,
        description,
        is_system,
        super_admin,
        permissions,
    })
}
