use crate::workflow::data::repository_trait::WorkflowRepositoryTrait;
use crate::workflow::data::Workflow;
use std::sync::Arc;
use uuid::Uuid;

// NOTE: Using API DTOs directly to simplify service signatures as requested.
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use cron::Schedule;
use std::str::FromStr;

pub struct WorkflowService {
    repo: Arc<dyn WorkflowRepositoryTrait>,
}

impl WorkflowService {
    pub fn new(repo: Arc<dyn WorkflowRepositoryTrait>) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    pub async fn get(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> {
        self.repo.get_by_uuid(uuid).await
    }

    pub async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        self.repo.create(req).await
    }

    pub async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        self.repo.update(uuid, req).await
    }

    pub async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.repo.delete(uuid).await
    }

    pub async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<(Vec<Workflow>, i64)> {
        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(limit, offset),
            self.repo.count_all()
        )?;
        Ok((items, total))
    }

    pub async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.repo
            .list_runs_paginated(workflow_uuid, limit, offset)
            .await
    }

    pub async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, String, String, Option<serde_json::Value>)>, i64)> {
        self.repo
            .list_run_logs_paginated(run_uuid, limit, offset)
            .await
    }

    pub async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool> {
        self.repo.run_exists(run_uuid).await
    }

    pub async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.repo.list_all_runs_paginated(limit, offset).await
    }
}
