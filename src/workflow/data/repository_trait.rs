use async_trait::async_trait;
use uuid::Uuid;

use super::Workflow;
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};

#[async_trait]
pub trait WorkflowRepositoryTrait: Send + Sync {
    async fn list_all(&self) -> anyhow::Result<Vec<Workflow>>;
    async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<Workflow>>;
    async fn count_all(&self) -> anyhow::Result<i64>;
    async fn get_by_uuid(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>>;
    async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid>;
    async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()>;
    async fn delete(&self, uuid: Uuid) -> anyhow::Result<()>;
    async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>>;
    async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> anyhow::Result<()>;
}
