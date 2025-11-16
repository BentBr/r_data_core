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
    /// Indicates if this workflow has a from.api source type (accepts POST, cron disabled)
    #[serde(default)]
    pub has_api_endpoint: bool,
    #[serde(default)]
    pub versioning_disabled: bool,
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
    #[serde(default)]
    pub versioning_disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
    #[serde(default)]
    pub versioning_disabled: bool,
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
    #[serde(default)]
    pub versioning_disabled: bool,
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

/// Multipart upload body for run-now file upload
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowRunUpload {
    /// CSV file to stage for this run
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}
