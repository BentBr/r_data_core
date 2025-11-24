#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::core::error::Error;
use crate::core::error::Result;
use crate::core::versioning::purger_trait::VersionPurger;

/// Repository for workflow versioning operations
pub struct WorkflowVersioningRepository {
    pool: PgPool,
}

impl WorkflowVersioningRepository {
    /// Create a new workflow versioning repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // PgPool is not const
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a pre-update snapshot for a workflow
    /// The snapshot's `created_by` is extracted from the JSON data (`updated_by` or `created_by`).
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Errors
    /// Returns an error if database operation fails
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
                .and_then(serde_json::Value::as_i64)
                .and_then(|v| i32::try_from(v).ok());
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
                "
                INSERT INTO workflow_versions (workflow_uuid, version_number, data, created_by)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (workflow_uuid, version_number) DO NOTHING
                ",
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
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Errors
    /// Returns an error if database query fails
    ///
    /// # Panics
    /// May panic if database row parsing fails
    pub async fn list_workflow_versions(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Vec<WorkflowVersionMeta>> {
        let rows = sqlx::query(
            "
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
            ",
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
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    /// * `version_number` - Version number to retrieve
    ///
    /// # Errors
    /// Returns an error if database query fails
    ///
    /// # Panics
    /// May panic if database row parsing fails
    pub async fn get_workflow_version(
        &self,
        workflow_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<WorkflowVersionPayload>> {
        let row = sqlx::query(
            "
            SELECT version_number, created_at, created_by, data
            FROM workflow_versions
            WHERE workflow_uuid = $1 AND version_number = $2
            ",
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
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Returns
    /// Tuple of (`version`, `updated_at`, `updated_by`, `updated_by_name`)
    ///
    /// # Errors
    /// Returns an error if database query fails
    ///
    /// # Panics
    /// May panic if database row parsing fails
    pub async fn get_current_workflow_metadata(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Option<(i32, time::OffsetDateTime, Option<Uuid>, Option<String>)>> {
        let row = sqlx::query(
            "
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
            ",
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

    /// Prune workflow versions older than the specified number of days
    ///
    /// # Arguments
    /// * `days` - Number of days to keep
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        let res = sqlx::query(
            "
            DELETE FROM workflow_versions
            WHERE created_at < NOW() - ($1::text || ' days')::interval
            ",
        )
        .bind(days.to_string())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }

    /// Prune workflow versions, keeping only the latest N versions per workflow
    ///
    /// # Arguments
    /// * `keep` - Number of latest versions to keep per workflow
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn prune_keep_latest_per_workflow(&self, keep: i32) -> Result<u64> {
        let res = sqlx::query(
            "
            WITH ranked AS (
                SELECT workflow_uuid,
                       version_number,
                       ROW_NUMBER() OVER (PARTITION BY workflow_uuid ORDER BY version_number DESC) AS rn
                FROM workflow_versions
            )
            DELETE FROM workflow_versions wv
            USING ranked r
            WHERE wv.workflow_uuid = r.workflow_uuid
              AND wv.version_number = r.version_number
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
impl VersionPurger for WorkflowVersioningRepository {
    fn repository_name(&self) -> &'static str {
        "workflows"
    }

    async fn prune_older_than_days(&self, days: i32) -> crate::core::error::Result<u64> {
        Self::prune_older_than_days(self, days)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))
    }

    async fn prune_keep_latest(&self, keep: i32) -> crate::core::error::Result<u64> {
        self.prune_keep_latest_per_workflow(keep)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))
    }
}

/// Metadata about a workflow version
#[derive(Debug, Clone)]
pub struct WorkflowVersionMeta {
    /// Version number
    pub version_number: i32,
    /// When the version was created
    pub created_at: time::OffsetDateTime,
    /// UUID of the user who created this version
    pub created_by: Option<Uuid>,
    /// Name of the user who created this version
    pub created_by_name: Option<String>,
}

/// Payload for a workflow version
#[derive(Debug, Clone)]
pub struct WorkflowVersionPayload {
    /// Version number
    pub version_number: i32,
    /// When the version was created
    pub created_at: time::OffsetDateTime,
    /// UUID of the user who created this version
    pub created_by: Option<Uuid>,
    /// The workflow data at this version
    pub data: serde_json::Value,
}

