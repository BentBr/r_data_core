use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Repository for entity definition versioning operations
pub struct EntityDefinitionVersioningRepository {
    pool: PgPool,
}

impl EntityDefinitionVersioningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a pre-update snapshot for an entity definition
    /// The snapshot's created_by is extracted from the JSON data (updated_by or created_by).
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
            let ver: Option<i32> = data
                .get("version")
                .and_then(|v| v.as_i64())
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
                r#"
                INSERT INTO entity_definition_versions (definition_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (definition_uuid, version_number) DO NOTHING
                "#,
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
    /// The snapshot's created_by is extracted from the JSON data (updated_by or created_by).
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
            let ver: Option<i32> = data
                .get("version")
                .and_then(|v| v.as_i64())
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
                r#"
                INSERT INTO entity_definition_versions (definition_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (definition_uuid, version_number) DO NOTHING
                "#,
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
    pub async fn list_definition_versions(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Vec<EntityDefinitionVersionMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                edv.version_number,
                edv.created_at,
                edv.created_by,
                COALESCE(
                    TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')),
                    au.username,
                    w.name
                ) AS created_by_name
            FROM entity_definition_versions edv
            LEFT JOIN admin_users au ON edv.created_by = au.uuid
            LEFT JOIN workflow_runs wr ON edv.created_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE edv.definition_uuid = $1
            ORDER BY edv.version_number DESC
            "#,
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
    pub async fn get_definition_version(
        &self,
        definition_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityDefinitionVersionPayload>> {
        let row = sqlx::query(
            r#"
            SELECT version_number, created_at, created_by, data
            FROM entity_definition_versions
            WHERE definition_uuid = $1 AND version_number = $2
            "#,
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
    pub async fn get_current_definition_metadata(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Option<(i32, time::OffsetDateTime, Option<Uuid>, Option<String>)>> {
        let row = sqlx::query(
            r#"
            SELECT 
                ed.version,
                ed.updated_at,
                ed.updated_by,
                COALESCE(
                    TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')),
                    au.username,
                    w.name
                ) AS updated_by_name
            FROM entity_definitions ed
            LEFT JOIN admin_users au ON ed.updated_by = au.uuid
            LEFT JOIN workflow_runs wr ON ed.updated_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE ed.uuid = $1
            "#,
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
