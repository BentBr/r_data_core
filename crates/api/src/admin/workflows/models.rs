#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// Note: WorkflowKind is imported from the main crate's workflow module
// This is a temporary dependency until workflow is migrated to a crate
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowSummary {
    pub uuid: Uuid,
    pub name: String,
    #[serde(rename = "kind")]
    pub kind: String, // Will be WorkflowKind once migrated
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
    #[serde(rename = "kind")]
    pub kind: String, // Will be WorkflowKind once migrated
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
    #[serde(default)]
    pub versioning_disabled: bool,
}

// Re-export from workflow crate
pub use r_data_core_workflow::data::requests::CreateWorkflowRequest;

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWorkflowResponse {
    pub uuid: Uuid,
}

// Re-export from workflow crate
pub use r_data_core_workflow::data::requests::UpdateWorkflowRequest;

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowRunSummary {
    pub uuid: Uuid,
    pub status: String,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub processed_items: Option<i64>,
    pub failed_items: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct WorkflowVersionMeta {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct WorkflowVersionPayload {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}

