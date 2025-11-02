use crate::workflow::data::WorkflowKind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowSummary {
    pub uuid: Uuid,
    pub name: String,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowDetail {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowResponse {
    pub uuid: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowRunSummary {
    pub uuid: Uuid,
    pub status: String,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub processed_items: Option<i64>,
    pub failed_items: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowRunLogDto {
    pub uuid: Uuid,
    pub ts: String,
    pub level: String,
    pub message: String,
    pub meta: Option<serde_json::Value>,
}
