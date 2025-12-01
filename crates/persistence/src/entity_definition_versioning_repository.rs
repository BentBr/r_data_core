use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity_definition_versioning_repository_trait::EntityDefinitionVersioningRepositoryTrait;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use r_data_core_core::versioning::purger_trait::VersionPurger;

/// Repository for entity definition versioning operations
pub struct EntityDefinitionVersioningRepository {
    pool: PgPool,
}

impl EntityDefinitionVersioningRepository {
    /// Create a new entity definition versioning repository
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a pre-update snapshot for an entity definition
    /// The snapshot's `created_by` is extracted from the JSON data (`updated_by` or `created_by`).
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn snapshot_pre_update(&self, definition_uuid: Uuid) -> Result<()> {
        // Get current definition data as JSON (includes version, updated_by, and created_by)
        let current_json: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(t) FROM (SELECT * FROM entity_definitions WHERE uuid = $1) t",
        )
        .bind(definition_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        if let Some(data) = current_json {
            // Extract version and creator from JSON
            // Cast i64 to i32 - version numbers are small enough
            #[allow(clippy::cast_possible_truncation)]
            let ver: Option<i32> = data
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .map(|v| v as i32);
            let Some(version) = ver else {
                return Ok(()); // No definition to snapshot
            };

            // Extract updated_by or created_by from the JSON data
            let snapshot_created_by = data
                .get("updated_by")
                .or_else(|| data.get("created_by"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            // Insert snapshot into entity_definition_versions
            sqlx::query(
                "
                INSERT INTO entity_definition_versions (definition_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (definition_uuid, version_number) DO NOTHING
                ",
            )
            .bind(definition_uuid)
            .bind(version)
            .bind(data)
            .bind(snapshot_created_by)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;
        }

        Ok(())
    }

    /// Create a pre-update snapshot within a transaction
    /// This is an associated function (static method) since it doesn't require a repository instance.
    /// The snapshot's `created_by` is extracted from the JSON data (`updated_by` or `created_by`).
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn snapshot_pre_update_tx(
        tx: &mut Transaction<'_, Postgres>,
        definition_uuid: Uuid,
        _new_updated_by: Option<Uuid>, // Not used - we get the current state from JSON
    ) -> Result<()> {
        // Get current definition data as JSON (includes version, updated_by, and created_by)
        let current_json: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(t) FROM (SELECT * FROM entity_definitions WHERE uuid = $1) t",
        )
        .bind(definition_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(Error::Database)?;

        if let Some(data) = current_json {
            // Extract version and creator from JSON
            // Cast i64 to i32 - version numbers are small enough
            #[allow(clippy::cast_possible_truncation)]
            let ver: Option<i32> = data
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .map(|v| v as i32);
            let Some(version) = ver else {
                return Ok(()); // No definition to snapshot
            };

            // Extract updated_by or created_by from the JSON data
            let snapshot_created_by = data
                .get("updated_by")
                .or_else(|| data.get("created_by"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            // Insert snapshot into entity_definition_versions
            sqlx::query(
                "
                INSERT INTO entity_definition_versions (definition_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (definition_uuid, version_number) DO NOTHING
                ",
            )
            .bind(definition_uuid)
            .bind(version)
            .bind(data)
            .bind(snapshot_created_by)
            .execute(&mut **tx)
            .await
            .map_err(Error::Database)?;
        }

        Ok(())
    }

    /// List all versions for an entity definition
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_definition_versions(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Vec<EntityDefinitionVersionMeta>> {
        let rows = sqlx::query(
            "
            SELECT 
                edv.version_number,
                edv.created_at,
                edv.created_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username,
                    w.name
                ) AS created_by_name
            FROM entity_definition_versions edv
            LEFT JOIN admin_users au ON edv.created_by = au.uuid
            LEFT JOIN workflow_runs wr ON edv.created_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE edv.definition_uuid = $1
            ORDER BY edv.version_number DESC
            ",
        )
        .bind(definition_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(EntityDefinitionVersionMeta {
                version_number: r.try_get("version_number").unwrap(),
                created_at: r.try_get("created_at").unwrap(),
                created_by: r.try_get("created_by").ok(),
                created_by_name: r.try_get("created_by_name").ok(),
            });
        }
        Ok(out)
    }

    /// Get a specific version of an entity definition
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn get_definition_version(
        &self,
        definition_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityDefinitionVersionPayload>> {
        let row = sqlx::query(
            "
            SELECT version_number, created_at, created_by, data
            FROM entity_definition_versions
            WHERE definition_uuid = $1 AND version_number = $2
            ",
        )
        .bind(definition_uuid)
        .bind(version_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| EntityDefinitionVersionPayload {
            version_number: r.try_get("version_number").unwrap(),
            created_at: r.try_get("created_at").unwrap(),
            created_by: r.try_get("created_by").ok(),
            data: r.try_get("data").unwrap(),
        }))
    }

    /// Get current entity definition metadata
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn get_current_definition_metadata(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Option<(i32, time::OffsetDateTime, Option<Uuid>, Option<String>)>> {
        let row = sqlx::query(
            "
            SELECT 
                ed.version,
                ed.updated_at,
                ed.updated_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username,
                    w.name
                ) AS updated_by_name
            FROM entity_definitions ed
            LEFT JOIN admin_users au ON ed.updated_by = au.uuid
            LEFT JOIN workflow_runs wr ON ed.updated_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE ed.uuid = $1
            ",
        )
        .bind(definition_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| {
            let version: i32 = r.try_get("version").unwrap();
            let updated_at: time::OffsetDateTime = r.try_get("updated_at").unwrap();
            let updated_by: Option<Uuid> = r.try_get("updated_by").ok();
            let updated_by_name: Option<String> = r.try_get("updated_by_name").ok();
            (version, updated_at, updated_by, updated_by_name)
        }))
    }

    /// Prune entity definition versions older than the specified number of days
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        let res = sqlx::query(
            "
            DELETE FROM entity_definition_versions
            WHERE created_at < NOW() - ($1::text || ' days')::interval
            ",
        )
        .bind(days.to_string())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }

    /// Prune entity definition versions, keeping only the latest N versions per definition
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn prune_keep_latest_per_definition(&self, keep: i32) -> Result<u64> {
        let res = sqlx::query(
            "
            WITH ranked AS (
                SELECT definition_uuid,
                       version_number,
                       ROW_NUMBER() OVER (PARTITION BY definition_uuid ORDER BY version_number DESC) AS rn
                FROM entity_definition_versions
            )
            DELETE FROM entity_definition_versions edv
            USING ranked r
            WHERE edv.definition_uuid = r.definition_uuid
              AND edv.version_number = r.version_number
              AND r.rn > $1
            ",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }
}

#[async_trait]
impl EntityDefinitionVersioningRepositoryTrait for EntityDefinitionVersioningRepository {
    async fn snapshot_pre_update(&self, definition_uuid: Uuid) -> Result<()> {
        Self::snapshot_pre_update(self, definition_uuid).await
    }

    async fn list_definition_versions(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Vec<EntityDefinitionVersionMeta>> {
        Self::list_definition_versions(self, definition_uuid).await
    }

    async fn get_definition_version(
        &self,
        definition_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityDefinitionVersionPayload>> {
        Self::get_definition_version(self, definition_uuid, version_number).await
    }

    async fn get_current_definition_metadata(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Option<(i32, OffsetDateTime, Option<Uuid>, Option<String>)>> {
        Self::get_current_definition_metadata(self, definition_uuid).await
    }

    async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        Self::prune_older_than_days(self, days).await
    }

    async fn prune_keep_latest_per_definition(&self, keep: i32) -> Result<u64> {
        Self::prune_keep_latest_per_definition(self, keep).await
    }
}

#[async_trait]
impl VersionPurger for EntityDefinitionVersioningRepository {
    fn repository_name(&self) -> &'static str {
        "entity_definitions"
    }

    async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        Self::prune_older_than_days(self, days)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))
    }

    async fn prune_keep_latest(&self, keep: i32) -> Result<u64> {
        self.prune_keep_latest_per_definition(keep)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct EntityDefinitionVersionMeta {
    pub version_number: i32,
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EntityDefinitionVersionPayload {
    pub version_number: i32,
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}
