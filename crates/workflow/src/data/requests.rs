#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use utoipa::ToSchema;

/// Request to create a new workflow
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct CreateWorkflowRequest {
    /// Workflow name
    pub name: String,
    /// Workflow description
    #[serde(default)]
    pub description: Option<String>,
    /// Workflow kind (consumer or provider)
    #[serde(rename = "kind")]
    pub kind: String,
    /// Whether the workflow is enabled
    pub enabled: bool,
    /// Cron schedule for the workflow
    #[serde(default)]
    pub schedule_cron: Option<String>,
    /// Workflow configuration
    #[ts(type = "unknown")]
    pub config: Value,
    /// Whether versioning is disabled
    #[serde(default)]
    pub versioning_disabled: bool,
}

/// Request to update an existing workflow
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct UpdateWorkflowRequest {
    /// Workflow name
    pub name: String,
    /// Workflow description
    #[serde(default)]
    pub description: Option<String>,
    /// Workflow kind (consumer or provider)
    #[serde(rename = "kind")]
    pub kind: String,
    /// Whether the workflow is enabled
    pub enabled: bool,
    /// Cron schedule for the workflow
    #[serde(default)]
    pub schedule_cron: Option<String>,
    /// Workflow configuration
    #[ts(type = "unknown")]
    pub config: Value,
    /// Whether versioning is disabled
    #[serde(default)]
    pub versioning_disabled: bool,
}
