#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::workflow_repository_trait::WorkflowRepositoryTrait;
use super::workflow_versioning_repository::WorkflowVersioningRepository;
use r_data_core_core::error::Result;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::{Workflow, WorkflowKind};
use std::str::FromStr;

pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get workflow UUID for a run UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_workflow_uuid_for_run(&self, run_uuid: Uuid) -> Result<Option<Uuid>> {
        self.get_workflow_uuid_for_run_internal(run_uuid).await
    }

    /// Get a workflow by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Workflow>> {
        let row = sqlx::query(
            "
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
            FROM workflows
            WHERE uuid = $1
            ",
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |r| {
                let uuid: Uuid = r
                    .try_get(0)
                    .map_err(r_data_core_core::error::Error::Database)?;
                let name: String = r
                    .try_get(1)
                    .map_err(r_data_core_core::error::Error::Database)?;
                let description: Option<String> = r.try_get(2).ok();
                let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
                let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
                let enabled: bool = r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true);
                let schedule_cron: Option<String> = r.try_get(5).ok();
                let config: serde_json::Value =
                    r.try_get(6).unwrap_or_else(|_| serde_json::json!({}));
                let versioning_disabled: bool = r
                    .try_get::<Option<bool>, _>(7)
                    .unwrap_or(Some(true))
                    .unwrap_or(true);
                let wf = Workflow {
                    uuid,
                    name,
                    description,
                    kind,
                    enabled,
                    schedule_cron,
                    config,
                    versioning_disabled,
                };
                Ok(Some(wf))
            },
        )
    }

    /// Create a new workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn create(&self, req: &CreateWorkflowRequest, created_by: Uuid) -> Result<Uuid> {
        let row = sqlx::query(
            "
            INSERT INTO workflows (name, description, kind, enabled, schedule_cron, config, versioning_disabled, created_by)
            VALUES ($1, $2, $3::workflow_kind, $4, $5, $6, $7, $8)
            RETURNING uuid
            ",
        )
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(&req.kind) // req.kind is already a String
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(&req.config)
        .bind(req.versioning_disabled)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("uuid")?)
    }

    /// Update an existing workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> Result<()> {
        // Pre-update snapshot of current workflow row
        let versioning_repo = WorkflowVersioningRepository::new(self.pool.clone());
        versioning_repo
            .snapshot_pre_update(uuid)
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Unknown(format!("Failed to snapshot workflow: {e}"))
            })?;

        sqlx::query(
            "
            UPDATE workflows
            SET name = $2, description = $3, kind = $4::workflow_kind, enabled = $5,
                schedule_cron = $6, config = $7, versioning_disabled = $8, updated_by = $9, version = version + 1, updated_at = NOW()
            WHERE uuid = $1
            ",
        )
        .bind(uuid)
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(&req.kind) // req.kind is already a String
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(&req.config)
        .bind(req.versioning_disabled)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete(&self, uuid: Uuid) -> r_data_core_core::error::Result<()> {
        sqlx::query("DELETE FROM workflows WHERE uuid = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_all(&self) -> r_data_core_core::error::Result<Vec<Workflow>> {
        let rows = sqlx::query(
            "
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
            FROM workflows
            ORDER BY name
            ",
        )
        .fetch_all(&self.pool)
        .await
        ?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            out.push(Workflow {
                uuid: r
                    .try_get(0)
                    .map_err(r_data_core_core::error::Error::Database)?,
                name: r
                    .try_get(1)
                    .map_err(r_data_core_core::error::Error::Database)?,
                description: r.try_get(2).ok(),
                kind,
                enabled: r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true),
                schedule_cron: r.try_get(5).ok(),
                config: r.try_get(6).unwrap_or_else(|_| serde_json::json!({})),
                versioning_disabled: r
                    .try_get::<Option<bool>, _>(7)
                    .unwrap_or(Some(false))
                    .unwrap_or(false),
            });
        }
        Ok(out)
    }

    /// List workflows with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> r_data_core_core::error::Result<Vec<Workflow>> {
        // Build ORDER BY clause - field is already validated and sanitized by route handler
        let order_by = sort_by.map_or_else(
            || "\"name\" ASC".to_string(),
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
                "
                SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
                FROM workflows
                ORDER BY {order_by} OFFSET $1
                "
            )
        } else {
            format!(
                "
                SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
                FROM workflows
                ORDER BY {order_by} LIMIT $1 OFFSET $2
                "
            )
        };

        let mut query_builder = sqlx::query(&query);
        if limit == i64::MAX {
            query_builder = query_builder.bind(offset);
        } else {
            query_builder = query_builder.bind(limit).bind(offset);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r
                .try_get(0)
                .map_err(r_data_core_core::error::Error::Database)?;
            let name: String = r
                .try_get(1)
                .map_err(r_data_core_core::error::Error::Database)?;
            let description: Option<String> = r.try_get(2).ok();
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            let enabled: bool = r
                .try_get::<Option<bool>, _>(4)
                .unwrap_or(Some(true))
                .unwrap_or(true);
            let schedule_cron: Option<String> = r.try_get(5).ok();
            let config: serde_json::Value = r.try_get(6).unwrap_or_else(|_| serde_json::json!({}));
            let versioning_disabled: bool = r
                .try_get::<Option<bool>, _>(7)
                .unwrap_or(Some(false))
                .unwrap_or(false);
            out.push(Workflow {
                uuid,
                name,
                description,
                kind,
                enabled,
                schedule_cron,
                config,
                versioning_disabled,
            });
        }
        Ok(out)
    }

    /// Count all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_all(&self) -> r_data_core_core::error::Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS cnt FROM workflows")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get::<i64, _>("cnt")?)
    }

    /// Check if a workflow config has from.api source type (accepts POST, cron disabled)
    /// or to.format.output.mode === 'api' (exports via GET, cron disabled)
    fn check_has_api_endpoint(config: &Value) -> bool {
        if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
            for step in steps {
                // Check for from.api source type (accepts POST, cron disabled)
                if let Some(from) = step.get("from") {
                    if let Some(source) = from
                        .get("source")
                        .or_else(|| from.get("format").and_then(|f| f.get("source")))
                    {
                        if let Some(source_type) =
                            source.get("source_type").and_then(|v| v.as_str())
                        {
                            if source_type == "api" {
                                // from.api without endpoint field = accepts POST
                                if let Some(config_obj) =
                                    source.get("config").and_then(|v| v.as_object())
                                {
                                    if !config_obj.contains_key("endpoint") {
                                        return true;
                                    }
                                } else {
                                    // No config object or empty config = accepts POST
                                    return true;
                                }
                            }
                        }
                    }
                }

                // Check for to.format.output.mode === 'api' (exports via GET, cron disabled)
                if let Some(to) = step.get("to") {
                    if let Some(to_type) = to.get("type").and_then(|v| v.as_str()) {
                        if to_type == "format" {
                            if let Some(output) = to.get("output") {
                                // Check if output is a string "api" or object with mode: "api"
                                if output.as_str() == Some("api") {
                                    return true;
                                }
                                if let Some(output_obj) = output.as_object() {
                                    if output_obj.get("mode").and_then(|v| v.as_str())
                                        == Some("api")
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// List scheduled workflow consumers
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_scheduled_consumers(
        &self,
    ) -> r_data_core_core::error::Result<Vec<(Uuid, String)>> {
        // Fetch workflows with their config to check for from.api source type
        let rows = sqlx::query(
            "SELECT uuid, schedule_cron, config FROM workflows WHERE enabled = true AND kind = 'consumer'::workflow_kind AND schedule_cron IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        ?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r
                .try_get(0)
                .map_err(r_data_core_core::error::Error::Database)?;
            let cron: String = r
                .try_get::<Option<String>, _>(1)
                .ok()
                .flatten()
                .unwrap_or_default();
            let config: Value = r.try_get(2).unwrap_or_else(|_| serde_json::json!({}));

            // Exclude workflows with from.api source type (they accept POST, not cron)
            if !Self::check_has_api_endpoint(&config) {
                out.push((uuid, cron));
            }
        }
        Ok(out)
    }

    /// Insert a queued workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn insert_run_queued(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Uuid,
    ) -> r_data_core_core::error::Result<Uuid> {
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
    pub async fn list_queued_runs(&self, limit: i64) -> r_data_core_core::error::Result<Vec<Uuid>> {
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
    pub async fn mark_run_running(&self, run_uuid: Uuid) -> r_data_core_core::error::Result<()> {
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
    ) -> r_data_core_core::error::Result<()> {
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
    pub async fn mark_run_failure(
        &self,
        run_uuid: Uuid,
        message: &str,
    ) -> r_data_core_core::error::Result<()> {
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
    pub async fn get_run_status(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Option<String>> {
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
    ) -> r_data_core_core::error::Result<()> {
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

    /// Insert raw items for a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn insert_raw_items(
        &self,
        _workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> r_data_core_core::error::Result<i64> {
        // Determine next sequence number for this run
        let start_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq_no), 0) FROM workflow_raw_items WHERE workflow_run_uuid = $1",
        )
        .bind(run_uuid)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let mut count: i64 = 0;
        for (idx, payload) in payloads.into_iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let seq_no = start_seq + (idx as i64) + 1;
            sqlx::query(
                "
                INSERT INTO workflow_raw_items (workflow_run_uuid, seq_no, payload, status)
                VALUES ($1, $2, $3, 'queued')
                ",
            )
            .bind(run_uuid)
            .bind(seq_no)
            .bind(payload)
            .execute(&self.pool)
            .await?;
            count += 1;
        }
        Ok(count)
    }

    /// Count raw items for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_raw_items_for_run(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS cnt FROM workflow_raw_items WHERE workflow_run_uuid = $1",
        )
        .bind(run_uuid)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get::<i64, _>("cnt")?)
    }

    /// Mark raw items as processed for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn mark_raw_items_processed(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        sqlx::query("UPDATE workflow_raw_items SET status = 'processed' WHERE workflow_run_uuid = $1 AND status = 'queued'")
            .bind(run_uuid)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Fetch staged raw items for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> r_data_core_core::error::Result<Vec<(Uuid, serde_json::Value)>> {
        let rows = sqlx::query(
            "
            SELECT uuid, payload
            FROM workflow_raw_items
            WHERE workflow_run_uuid = $1 AND status = 'queued'
            ORDER BY seq_no ASC
            LIMIT $2
            ",
        )
        .bind(run_uuid)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r.try_get("uuid")?;
            let payload: serde_json::Value = r.try_get("payload")?;
            out.push((uuid, payload));
        }
        Ok(out)
    }

    /// Set the status of a raw item
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> r_data_core_core::error::Result<()> {
        sqlx::query(
            "
            UPDATE workflow_raw_items
            SET status = $2::data_raw_item_status, error = $3
            WHERE uuid = $1
            ",
        )
        .bind(item_uuid)
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get workflow UUID for a run UUID (internal implementation)
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_workflow_uuid_for_run_internal(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Option<Uuid>> {
        let row = sqlx::query("SELECT workflow_uuid FROM workflow_runs WHERE uuid = $1")
            .bind(run_uuid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.try_get::<Uuid, _>("workflow_uuid").ok()))
    }
}

#[async_trait::async_trait]
impl WorkflowRepositoryTrait for WorkflowRepository {
    async fn list_all(&self) -> r_data_core_core::error::Result<Vec<Workflow>> {
        self.list_all().await
    }
    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> r_data_core_core::error::Result<Vec<Workflow>> {
        self.list_paginated(limit, offset, sort_by, sort_order)
            .await
    }
    async fn count_all(&self) -> r_data_core_core::error::Result<i64> {
        self.count_all().await
    }
    async fn get_by_uuid(&self, uuid: Uuid) -> r_data_core_core::error::Result<Option<Workflow>> {
        self.get_by_uuid(uuid).await
    }
    async fn create(
        &self,
        req: &CreateWorkflowRequest,
        created_by: Uuid,
    ) -> r_data_core_core::error::Result<Uuid> {
        self.create(req, created_by).await
    }
    async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        self.update(uuid, req, updated_by).await
    }
    async fn delete(&self, uuid: Uuid) -> r_data_core_core::error::Result<()> {
        self.delete(uuid).await
    }
    async fn list_scheduled_consumers(
        &self,
    ) -> r_data_core_core::error::Result<Vec<(Uuid, String)>> {
        self.list_scheduled_consumers().await
    }
    async fn insert_run_queued(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Uuid,
    ) -> r_data_core_core::error::Result<Uuid> {
        self.insert_run_queued(workflow_uuid, trigger_id).await
    }

    async fn mark_run_running(&self, run_uuid: Uuid) -> r_data_core_core::error::Result<()> {
        self.mark_run_running(run_uuid).await
    }

    async fn mark_run_success(
        &self,
        run_uuid: Uuid,
        processed: i64,
        failed: i64,
    ) -> r_data_core_core::error::Result<()> {
        self.mark_run_success(run_uuid, processed, failed).await
    }

    async fn mark_run_failure(
        &self,
        run_uuid: Uuid,
        message: &str,
    ) -> r_data_core_core::error::Result<()> {
        self.mark_run_failure(run_uuid, message).await
    }

    async fn get_run_status(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Option<String>> {
        self.get_run_status(run_uuid).await
    }

    async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> r_data_core_core::error::Result<(
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
        .await
        ?;

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

    async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> r_data_core_core::error::Result<(
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

    async fn run_exists(&self, run_uuid: Uuid) -> r_data_core_core::error::Result<bool> {
        let row = sqlx::query("SELECT 1 FROM workflow_runs WHERE uuid = $1")
            .bind(run_uuid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> r_data_core_core::error::Result<(
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

    async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> r_data_core_core::error::Result<()> {
        self.insert_run_log(run_uuid, level, message, meta).await
    }

    async fn insert_raw_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> r_data_core_core::error::Result<i64> {
        self.insert_raw_items(workflow_uuid, run_uuid, payloads)
            .await
    }

    async fn count_raw_items_for_run(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<i64> {
        self.count_raw_items_for_run(run_uuid).await
    }

    async fn mark_raw_items_processed(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        self.mark_raw_items_processed(run_uuid).await
    }

    async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> r_data_core_core::error::Result<Vec<(Uuid, serde_json::Value)>> {
        self.fetch_staged_raw_items(run_uuid, limit).await
    }

    async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> r_data_core_core::error::Result<()> {
        self.set_raw_item_status(item_uuid, status, error).await
    }

    async fn get_workflow_uuid_for_run(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Option<Uuid>> {
        self.get_workflow_uuid_for_run_internal(run_uuid).await
    }
}

/// Get provider workflow configuration
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn get_provider_config(
    pool: &PgPool,
    uuid: Uuid,
) -> r_data_core_core::error::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        "SELECT config FROM workflows WHERE uuid = $1 AND kind = 'provider'::workflow_kind AND enabled = true",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await?;

    let cfg = row.map(|r| r.get::<serde_json::Value, _>("config"));
    Ok(cfg)
}
