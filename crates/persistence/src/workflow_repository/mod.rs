#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod crud;
mod raw_items;
mod runs;

use sqlx::PgPool;
use uuid::Uuid;

use super::workflow_repository_trait::WorkflowRepositoryTrait;
use r_data_core_core::error::Result;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::Workflow;

pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl WorkflowRepositoryTrait for WorkflowRepository {
    async fn list_all(&self) -> Result<Vec<Workflow>> {
        self.list_all().await
    }
    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Workflow>> {
        self.list_paginated(limit, offset, sort_by, sort_order)
            .await
    }
    async fn count_all(&self) -> Result<i64> {
        self.count_all().await
    }
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Workflow>> {
        self.get_by_uuid(uuid).await
    }
    async fn create(&self, req: &CreateWorkflowRequest, created_by: Uuid) -> Result<Uuid> {
        self.create(req, created_by).await
    }
    async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> Result<()> {
        self.update(uuid, req, updated_by).await
    }
    async fn delete(&self, uuid: Uuid) -> Result<()> {
        self.delete(uuid).await
    }
    async fn list_scheduled_consumers(&self) -> Result<Vec<(Uuid, String)>> {
        self.list_scheduled_consumers().await
    }
    async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> Result<Uuid> {
        self.insert_run_queued(workflow_uuid, trigger_id).await
    }
    async fn mark_run_running(&self, run_uuid: Uuid) -> Result<()> {
        self.mark_run_running(run_uuid).await
    }
    async fn mark_run_success(&self, run_uuid: Uuid, processed: i64, failed: i64) -> Result<()> {
        self.mark_run_success(run_uuid, processed, failed).await
    }
    async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> Result<()> {
        self.mark_run_failure(run_uuid, message).await
    }
    async fn get_run_status(&self, run_uuid: Uuid) -> Result<Option<String>> {
        self.get_run_status(run_uuid).await
    }
    async fn list_runs_paginated(
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
        self.list_runs_paginated(workflow_uuid, limit, offset).await
    }
    async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(
        Vec<(Uuid, String, String, String, Option<serde_json::Value>)>,
        i64,
    )> {
        self.list_run_logs_paginated(run_uuid, limit, offset).await
    }
    async fn run_exists(&self, run_uuid: Uuid) -> Result<bool> {
        self.run_exists(run_uuid).await
    }
    async fn list_all_runs_paginated(
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
        self.list_all_runs_paginated(limit, offset).await
    }
    async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> Result<()> {
        self.insert_run_log(run_uuid, level, message, meta).await
    }
    async fn insert_raw_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> Result<i64> {
        self.insert_raw_items(workflow_uuid, run_uuid, payloads)
            .await
    }
    async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> Result<i64> {
        self.count_raw_items_for_run(run_uuid).await
    }
    async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> Result<()> {
        self.mark_raw_items_processed(run_uuid).await
    }
    async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> Result<Vec<(Uuid, serde_json::Value)>> {
        self.fetch_staged_raw_items(run_uuid, limit).await
    }
    async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        self.set_raw_item_status(item_uuid, status, error).await
    }
    async fn get_workflow_uuid_for_run(&self, run_uuid: Uuid) -> Result<Option<Uuid>> {
        self.get_workflow_uuid_for_run_internal(run_uuid).await
    }
}

/// Get provider workflow configuration
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn get_provider_config(pool: &PgPool, uuid: Uuid) -> Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        "SELECT config FROM workflows WHERE uuid = $1 AND kind = 'provider'::workflow_kind AND enabled = true",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await?;

    let cfg = row.map(|r| sqlx::Row::get::<serde_json::Value, _>(&r, "config"));
    Ok(cfg)
}
