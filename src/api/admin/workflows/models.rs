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
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub consumer_config: Option<serde_json::Value>,
    pub provider_config: Option<serde_json::Value>,
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
    pub consumer_config: Option<serde_json::Value>,
    pub provider_config: Option<serde_json::Value>,
}
