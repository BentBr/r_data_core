#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::Row;
use uuid::Uuid;

use super::WorkflowRepository;
use r_data_core_core::error::Result;

impl WorkflowRepository {
    /// Get workflow UUID for a run UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_workflow_uuid_for_run(&self, run_uuid: Uuid) -> Result<Option<Uuid>> {
        self.get_workflow_uuid_for_run_internal(run_uuid).await
    }

    /// Insert a queued workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> Result<Uuid> {
        let row = sqlx::query(
            "INSERT INTO workflow_runs (workflow_uuid, status, trigger_id) VALUES ($1, 'queued', $2) RETURNING uuid",
        )
        .bind(workflow_uuid)
        .bind(trigger_id)
        .fetch_one(&self.pool)
        .await
        ?;
        Ok(row.try_get("uuid")?)
    }

    /// List queued workflow runs
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_queued_runs(&self, limit: i64) -> Result<Vec<Uuid>> {
        let rows = sqlx::query("SELECT uuid FROM workflow_runs WHERE status = 'queued' ORDER BY queued_at ASC LIMIT $1")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            ?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(r.try_get::<Uuid, _>("uuid")?);
        }
        Ok(out)
    }

    /// Mark a workflow run as running
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn mark_run_running(&self, run_uuid: Uuid) -> Result<()> {
        sqlx::query("UPDATE workflow_runs SET status = 'running', started_at = NOW() WHERE uuid = $1 AND status = 'queued'")
            .bind(run_uuid)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Mark a workflow run as successful
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn mark_run_success(
        &self,
        run_uuid: Uuid,
        processed: i64,
        failed: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE workflow_runs SET status = 'success', finished_at = NOW(), processed_items = $2, failed_items = $3 WHERE uuid = $1")
            .bind(run_uuid)
            .bind(processed)
            .bind(failed)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Mark a workflow run as failed
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> Result<()> {
        sqlx::query("UPDATE workflow_runs SET status = 'failed', finished_at = NOW(), error = $2 WHERE uuid = $1")
            .bind(run_uuid)
            .bind(message)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Get run status
    ///
    /// # Errors
    /// Returns an error if query fails
    pub async fn get_run_status(&self, run_uuid: Uuid) -> Result<Option<String>> {
        let row = sqlx::query("SELECT status::text FROM workflow_runs WHERE uuid = $1")
            .bind(run_uuid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.try_get::<String, _>("status").ok()))
    }

    /// Insert a log entry for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> Result<()> {
        sqlx::query("INSERT INTO workflow_run_logs (run_uuid, level, message, meta) VALUES ($1, $2, $3, $4)")
            .bind(run_uuid)
            .bind(level)
            .bind(message)
            .bind(meta)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Get workflow UUID for a run UUID (internal implementation)
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_workflow_uuid_for_run_internal(&self, run_uuid: Uuid) -> Result<Option<Uuid>> {
        let row = sqlx::query("SELECT workflow_uuid FROM workflow_runs WHERE uuid = $1")
            .bind(run_uuid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.try_get::<Uuid, _>("workflow_uuid").ok()))
    }

    /// List runs for a workflow with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )> {
        let runs = sqlx::query(
            r#"
            SELECT uuid, status::text, to_char(queued_at, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS queued_at,
                   to_char(started_at, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS started_at,
                   to_char(finished_at, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS finished_at,
                   processed_items::bigint AS processed_items, failed_items::bigint AS failed_items
            FROM workflow_runs
            WHERE workflow_uuid = $1
            ORDER BY queued_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(workflow_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total_row =
            sqlx::query("SELECT COUNT(*) AS cnt FROM workflow_runs WHERE workflow_uuid = $1")
                .bind(workflow_uuid)
                .fetch_one(&self.pool)
                .await?;
        let total: i64 = total_row.try_get("cnt")?;

        let mut out = Vec::with_capacity(runs.len());
        for r in runs {
            out.push((
                r.try_get("uuid")?,
                r.try_get("status")?,
                r.try_get::<Option<String>, _>("queued_at")?,
                r.try_get::<Option<String>, _>("finished_at")?,
                r.try_get::<Option<i64>, _>("processed_items")?,
                r.try_get::<Option<i64>, _>("failed_items")?,
            ));
        }
        Ok((out, total))
    }

    /// List run logs with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(
        Vec<(Uuid, String, String, String, Option<serde_json::Value>)>,
        i64,
    )> {
        let rows = sqlx::query(
            r#"
            SELECT uuid, to_char(ts, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS ts, level, message, meta
            FROM workflow_run_logs
            WHERE run_uuid = $1
            ORDER BY ts DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(run_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total_row =
            sqlx::query("SELECT COUNT(*) AS cnt FROM workflow_run_logs WHERE run_uuid = $1")
                .bind(run_uuid)
                .fetch_one(&self.pool)
                .await?;
        let total: i64 = total_row.try_get("cnt")?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push((
                r.try_get("uuid")?,
                r.try_get("ts")?,
                r.try_get("level")?,
                r.try_get("message")?,
                r.try_get("meta").ok(),
            ));
        }
        Ok((out, total))
    }

    /// Check if a run exists
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn run_exists(&self, run_uuid: Uuid) -> Result<bool> {
        let row = sqlx::query("SELECT 1 FROM workflow_runs WHERE uuid = $1")
            .bind(run_uuid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    /// List all runs with pagination (across all workflows)
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )> {
        let runs = sqlx::query(
            r#"
            SELECT uuid, status::text,
                   to_char(queued_at, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS queued_at,
                   to_char(finished_at, 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS finished_at,
                   processed_items::bigint AS processed_items, failed_items::bigint AS failed_items
            FROM workflow_runs
            ORDER BY queued_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total_row = sqlx::query("SELECT COUNT(*) AS cnt FROM workflow_runs")
            .fetch_one(&self.pool)
            .await?;
        let total: i64 = total_row.try_get("cnt")?;

        let mut out = Vec::with_capacity(runs.len());
        for r in runs {
            out.push((
                r.try_get("uuid")?,
                r.try_get("status")?,
                r.try_get::<Option<String>, _>("queued_at")?,
                r.try_get::<Option<String>, _>("finished_at")?,
                r.try_get::<Option<i64>, _>("processed_items")?,
                r.try_get::<Option<i64>, _>("failed_items")?,
            ));
        }
        Ok((out, total))
    }
}
