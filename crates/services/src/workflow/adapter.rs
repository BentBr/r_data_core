#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::WorkflowRepositoryTrait as WorkflowRepositoryTraitDef;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};

pub struct WorkflowRepositoryAdapter {
    inner: WorkflowRepository,
}

impl WorkflowRepositoryAdapter {
    #[must_use]
    pub const fn new(inner: WorkflowRepository) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl WorkflowRepositoryTraitDef for WorkflowRepositoryAdapter {
    async fn list_all(&self) -> anyhow::Result<Vec<r_data_core_workflow::data::Workflow>> {
        self.inner.list_all().await
    }

    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<r_data_core_workflow::data::Workflow>> {
        self.inner.list_paginated(limit, offset).await
    }

    async fn count_all(&self) -> anyhow::Result<i64> {
        self.inner.count_all().await
    }

    async fn get_by_uuid(
        &self,
        uuid: Uuid,
    ) -> anyhow::Result<Option<r_data_core_workflow::data::Workflow>> {
        self.inner.get_by_uuid(uuid).await
    }

    async fn create(&self, req: &CreateWorkflowRequest, created_by: Uuid) -> anyhow::Result<Uuid> {
        self.inner.create(req, created_by).await
    }

    async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> anyhow::Result<()> {
        self.inner.update(uuid, req, updated_by).await
    }

    async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.inner.delete(uuid).await
    }

    async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>> {
        self.inner.list_scheduled_consumers().await
    }

    async fn insert_run_queued(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Uuid,
    ) -> anyhow::Result<Uuid> {
        self.inner
            .insert_run_queued(workflow_uuid, trigger_id)
            .await
    }

    async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
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
        self.inner
            .list_runs_paginated(workflow_uuid, limit, offset)
            .await
    }

    async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(Uuid, String, String, String, Option<serde_json::Value>)>,
        i64,
    )> {
        self.inner
            .list_run_logs_paginated(run_uuid, limit, offset)
            .await
    }

    async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool> {
        self.inner.run_exists(run_uuid).await
    }

    async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
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
        self.inner.list_all_runs_paginated(limit, offset).await
    }

    async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        self.inner
            .insert_run_log(run_uuid, level, message, meta)
            .await
    }

    async fn insert_raw_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> anyhow::Result<i64> {
        self.inner
            .insert_raw_items(workflow_uuid, run_uuid, payloads)
            .await
    }

    async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> anyhow::Result<i64> {
        self.inner.count_raw_items_for_run(run_uuid).await
    }

    async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> anyhow::Result<()> {
        self.inner.mark_raw_items_processed(run_uuid).await
    }

    async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<(Uuid, serde_json::Value)>> {
        self.inner.fetch_staged_raw_items(run_uuid, limit).await
    }

    async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        self.inner
            .set_raw_item_status(item_uuid, status, error)
            .await
    }

    async fn mark_run_success(
        &self,
        run_uuid: Uuid,
        processed: i64,
        failed: i64,
    ) -> anyhow::Result<()> {
        self.inner
            .mark_run_success(run_uuid, processed, failed)
            .await
    }

    async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> anyhow::Result<()> {
        self.inner.mark_run_failure(run_uuid, message).await
    }

    async fn get_workflow_uuid_for_run(&self, run_uuid: Uuid) -> anyhow::Result<Option<Uuid>> {
        self.inner
            .get_workflow_uuid_for_run_internal(run_uuid)
            .await
    }
}
