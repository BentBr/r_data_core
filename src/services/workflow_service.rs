use crate::workflow::data::repository_trait::WorkflowRepositoryTrait;
use crate::workflow::data::Workflow;
use std::sync::Arc;
use uuid::Uuid;

// NOTE: Using API DTOs directly to simplify service signatures as requested.
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};

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
        self.repo.create(req).await
    }

    pub async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
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
}
