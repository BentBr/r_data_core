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
    async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> anyhow::Result<Uuid>;
    async fn list_runs_paginated(&self, workflow_uuid: Uuid, limit: i64, offset: i64) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)>;
    async fn list_run_logs_paginated(&self, run_uuid: Uuid, limit: i64, offset: i64) -> anyhow::Result<(Vec<(Uuid, String, String, String, Option<serde_json::Value>)>, i64)>;
    async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool>;
    async fn list_all_runs_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)>;
    async fn insert_run_log(&self, run_uuid: Uuid, level: &str, message: &str, meta: Option<serde_json::Value>) -> anyhow::Result<()>;
    async fn insert_raw_items(&self, workflow_uuid: Uuid, run_uuid: Uuid, payloads: Vec<serde_json::Value>) -> anyhow::Result<i64>;
    async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> anyhow::Result<i64>;
    async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> anyhow::Result<()>;
}
