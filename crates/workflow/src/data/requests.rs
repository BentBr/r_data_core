#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;

/// Request to create a new workflow
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow kind (consumer or provider)
    #[serde(rename = "kind")]
    pub kind: String,
    /// Whether the workflow is enabled
    pub enabled: bool,
    /// Cron schedule for the workflow
    pub schedule_cron: Option<String>,
    /// Workflow configuration
    pub config: Value,
    /// Whether versioning is disabled
    #[serde(default)]
    pub versioning_disabled: bool,
}

/// Request to update an existing workflow
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWorkflowRequest {
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow kind (consumer or provider)
    #[serde(rename = "kind")]
    pub kind: String,
    /// Whether the workflow is enabled
    pub enabled: bool,
    /// Cron schedule for the workflow
    pub schedule_cron: Option<String>,
    /// Workflow configuration
    pub config: Value,
    /// Whether versioning is disabled
    #[serde(default)]
    pub versioning_disabled: bool,
}
