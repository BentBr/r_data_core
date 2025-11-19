use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Repository for workflow versioning operations
pub struct WorkflowVersioningRepository {
    pool: PgPool,
}

impl WorkflowVersioningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a pre-update snapshot for a workflow
    /// The snapshot's created_by is extracted from the JSON data (updated_by or created_by).
    pub async fn snapshot_pre_update(&self, workflow_uuid: Uuid) -> Result<()> {
        // Get current workflow data as JSON (includes version, updated_by, and created_by)
        let current_json: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(t) FROM (SELECT * FROM workflows WHERE uuid = $1) t",
        )
        .bind(workflow_uuid)
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
                return Ok(()); // No workflow to snapshot
            };

            // Extract updated_by or created_by from the JSON data
            let snapshot_created_by = data
                .get("updated_by")
                .or_else(|| data.get("created_by"))
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            // Insert snapshot into workflow_versions
            sqlx::query(
                r#"
                INSERT INTO workflow_versions (workflow_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (workflow_uuid, version_number) DO NOTHING
                "#,
            )
            .bind(workflow_uuid)
            .bind(version)
            .bind(data)
            .bind(snapshot_created_by)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;
        }

        Ok(())
    }

    /// List all versions for a workflow
    pub async fn list_workflow_versions(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Vec<WorkflowVersionMeta>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                wv.version_number,
                wv.created_at,
                wv.created_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username
                ) AS created_by_name
            FROM workflow_versions wv
            LEFT JOIN admin_users au ON wv.created_by = au.uuid
            WHERE wv.workflow_uuid = $1
            ORDER BY wv.version_number DESC
            "#,
        )
        .bind(workflow_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(WorkflowVersionMeta {
                version_number: r.try_get("version_number").unwrap(),
                created_at: r.try_get("created_at").unwrap(),
                created_by: r.try_get("created_by").ok(),
                created_by_name: r.try_get("created_by_name").ok(),
            });
        }
        Ok(out)
    }

    /// Get a specific version of a workflow
    pub async fn get_workflow_version(
        &self,
        workflow_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<WorkflowVersionPayload>> {
        let row = sqlx::query(
            r#"
            SELECT version_number, created_at, created_by, data
            FROM workflow_versions
            WHERE workflow_uuid = $1 AND version_number = $2
            "#,
        )
        .bind(workflow_uuid)
        .bind(version_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| WorkflowVersionPayload {
            version_number: r.try_get("version_number").unwrap(),
            created_at: r.try_get("created_at").unwrap(),
            created_by: r.try_get("created_by").ok(),
            data: r.try_get("data").unwrap(),
        }))
    }

    /// Get current workflow metadata
    pub async fn get_current_workflow_metadata(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Option<(i32, time::OffsetDateTime, Option<Uuid>, Option<String>)>> {
        let row = sqlx::query(
            r#"
            SELECT 
                w.version,
                w.updated_at,
                w.updated_by,
                COALESCE(
                    NULLIF(TRIM(COALESCE(au.first_name || ' ', '') || COALESCE(au.last_name, '')), ''),
                    au.username
                ) AS updated_by_name
            FROM workflows w
            LEFT JOIN admin_users au ON w.updated_by = au.uuid
            WHERE w.uuid = $1
            "#,
        )
        .bind(workflow_uuid)
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
pub struct WorkflowVersionMeta {
    pub version_number: i32,
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowVersionPayload {
    pub version_number: i32,
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}
