use sqlx::{PgPool, Row, Transaction, Postgres};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct EntityVersionMeta {
    pub version_number: i32,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Uuid>,
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
            SELECT version_number, created_at, created_by
            FROM entities_versions
            WHERE entity_uuid = $1
            ORDER BY version_number DESC
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
            out.push(EntityVersionMeta {
                version_number,
                created_at,
                created_by,
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
}
