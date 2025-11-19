use sqlx::{PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct EntityVersionMeta {
    pub version_number: i32,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EntityVersionPayload {
    pub version_number: i32,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}

pub struct VersionRepository {
    pool: PgPool,
}

impl VersionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_entity_versions(&self, entity_uuid: Uuid) -> Result<Vec<EntityVersionMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                ev.version_number,
                ev.created_at,
                ev.created_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username,
                    w.name
                ) AS created_by_name
            FROM entities_versions ev
            LEFT JOIN admin_users au ON ev.created_by = au.uuid
            LEFT JOIN workflow_runs wr ON ev.created_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE ev.entity_uuid = $1
            ORDER BY ev.version_number DESC
            "#,
        )
        .bind(entity_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut out: Vec<EntityVersionMeta> = Vec::with_capacity(rows.len());
        for r in rows {
            let version_number: i32 = r.try_get("version_number").unwrap();
            let created_at: time::OffsetDateTime = r.try_get("created_at").unwrap();
            let created_by: Option<Uuid> = r.try_get("created_by").ok();
            let created_by_name: Option<String> = r.try_get("created_by_name").ok();
            out.push(EntityVersionMeta {
                version_number,
                created_at,
                created_by,
                created_by_name,
            });
        }
        Ok(out)
    }

    pub async fn get_entity_version(
        &self,
        entity_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityVersionPayload>> {
        let row = sqlx::query(
            r#"
            SELECT version_number, created_at, created_by, data
            FROM entities_versions
            WHERE entity_uuid = $1 AND version_number = $2
            "#,
        )
        .bind(entity_uuid)
        .bind(version_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| {
            let version_number: i32 = r.try_get("version_number").unwrap();
            let created_at: time::OffsetDateTime = r.try_get("created_at").unwrap();
            let created_by: Option<Uuid> = r.try_get("created_by").ok();
            let data: serde_json::Value = r.try_get("data").unwrap();
            EntityVersionPayload {
                version_number,
                created_at,
                created_by,
                data,
            }
        }))
    }

    pub async fn insert_snapshot(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
        version_number: i32,
        data: serde_json::Value,
        created_by: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at, created_by)
            VALUES ($1, $2, $3, $4, NOW(), $5)
            ON CONFLICT (entity_uuid, version_number) DO NOTHING
            "#,
        )
        .bind(entity_uuid)
        .bind(entity_type)
        .bind(version_number)
        .bind(data)
        .bind(created_by)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(())
    }

    /// Insert a version snapshot within a transaction.
    /// This is an associated function (static method) since it doesn't require a VersionRepository instance.
    pub async fn insert_snapshot_tx(
        tx: &mut Transaction<'_, Postgres>,
        entity_uuid: Uuid,
        entity_type: &str,
        version_number: i32,
        data: serde_json::Value,
        created_by: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at, created_by)
            VALUES ($1, $2, $3, $4, NOW(), $5)
            ON CONFLICT (entity_uuid, version_number) DO NOTHING
            "#,
        )
        .bind(entity_uuid)
        .bind(entity_type)
        .bind(version_number)
        .bind(data)
        .bind(created_by)
        .execute(&mut **tx)
        .await
        .map_err(Error::Database)?;
        Ok(())
    }

    pub async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        let res = sqlx::query(
            r#"
            DELETE FROM entities_versions
            WHERE created_at < NOW() - ($1::text || ' days')::interval
            "#,
        )
        .bind(days.to_string())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }

    pub async fn prune_keep_latest_per_entity(&self, keep: i32) -> Result<u64> {
        let res = sqlx::query(
            r#"
            WITH ranked AS (
                SELECT entity_uuid,
                       version_number,
                       ROW_NUMBER() OVER (PARTITION BY entity_uuid ORDER BY version_number DESC) AS rn
                FROM entities_versions
            )
            DELETE FROM entities_versions ev
            USING ranked r
            WHERE ev.entity_uuid = r.entity_uuid
              AND ev.version_number = r.version_number
              AND r.rn > $1
            "#,
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }

    /// Get current entity metadata from entities_registry with resolved creator name
    pub async fn get_current_entity_metadata(
        &self,
        entity_uuid: Uuid,
    ) -> Result<Option<(i32, OffsetDateTime, Option<Uuid>, Option<String>)>> {
        let row = sqlx::query(
            r#"
            SELECT 
                er.version,
                er.updated_at,
                er.updated_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username,
                    w.name
                ) AS updated_by_name
            FROM entities_registry er
            LEFT JOIN admin_users au ON er.updated_by = au.uuid
            LEFT JOIN workflow_runs wr ON er.updated_by = wr.uuid
            LEFT JOIN workflows w ON wr.workflow_uuid = w.uuid
            WHERE er.uuid = $1
            "#,
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| {
            let version: i32 = r.try_get("version").unwrap();
            let updated_at: OffsetDateTime = r.try_get("updated_at").unwrap();
            let updated_by: Option<Uuid> = r.try_get("updated_by").ok();
            let updated_by_name: Option<String> = r.try_get("updated_by_name").ok();
            (version, updated_at, updated_by, updated_by_name)
        }))
    }

    /// Get current entity data as JSON from the entity view
    pub async fn get_current_entity_data(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
    ) -> Result<Option<serde_json::Value>> {
        use crate::entity::dynamic_entity::utils;
        let view_name = utils::get_view_name(entity_type);
        let current_json: Option<serde_json::Value> = sqlx::query_scalar(&format!(
            "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE uuid = $1) t",
            view_name
        ))
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(current_json)
    }

    /// Create a pre-update snapshot for a dynamic entity within a transaction.
    /// This is an associated function (static method) since it doesn't require a VersionRepository instance.
    /// The snapshot's created_by is set to the current updated_by (or created_by if updated_by is None).
    pub async fn snapshot_pre_update_tx(
        tx: &mut Transaction<'_, Postgres>,
        entity_uuid: Uuid,
    ) -> Result<()> {
        use crate::entity::dynamic_entity::utils;

        // Read current entity_type, version, updated_by, and created_by in a single query
        let row = sqlx::query("SELECT entity_type, version, updated_by, created_by FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_optional(&mut **tx)
            .await
            .map_err(Error::Database)?;

        let (entity_type, version, snapshot_created_by): (String, i32, Option<Uuid>) = match row {
            Some(r) => {
                let et: String = r.try_get("entity_type").map_err(Error::Database)?;
                let v: i32 = r.try_get("version").map_err(Error::Database)?;
                // Use updated_by if it exists, otherwise use created_by
                let updated_by: Option<Uuid> = r.try_get("updated_by").ok();
                let created_by: Option<Uuid> = r.try_get("created_by").ok();
                let snapshot_creator = updated_by.or(created_by);
                (et, v, snapshot_creator)
            }
            None => return Ok(()), // nothing to snapshot
        };

        // Build view name and read current row as JSON
        let view_name = utils::get_view_name(&entity_type);
        let current_json: Option<serde_json::Value> = sqlx::query_scalar(&format!(
            "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE uuid = $1) t",
            view_name
        ))
        .bind(entity_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(Error::Database)?;

        if let Some(data_json) = current_json {
            Self::insert_snapshot_tx(
                tx,
                entity_uuid,
                &entity_type,
                version,
                data_json,
                snapshot_created_by,
            )
            .await?;
        }

        Ok(())
    }
}
