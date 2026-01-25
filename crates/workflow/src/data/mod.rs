#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod adapters;
pub mod job_queue;
pub mod jobs;
pub mod requests;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use uuid::Uuid;

/// Workflow kind enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowKind {
    /// Consumer workflow (processes data)
    Consumer,
    /// Provider workflow (provides data)
    Provider,
}

impl Display for WorkflowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Consumer => write!(f, "consumer"),
            Self::Provider => write!(f, "provider"),
        }
    }
}

impl FromStr for WorkflowKind {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "consumer" => Ok(Self::Consumer),
            "provider" => Ok(Self::Provider),
            _ => Err("invalid workflow kind"),
        }
    }
}

/// Workflow run status enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// Run is queued
    Queued,
    /// Run is running
    Running,
    /// Run completed successfully
    Success,
    /// Run failed
    Failed,
    /// Run was cancelled
    Cancelled,
}

/// Workflow data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow UUID
    pub uuid: Uuid,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: Option<String>,
    /// Workflow kind
    pub kind: WorkflowKind,
    /// Whether the workflow is enabled
    pub enabled: bool,
    /// Cron schedule for the workflow
    pub schedule_cron: Option<String>,
    /// Workflow configuration
    pub config: serde_json::Value,
    /// Whether versioning is disabled
    pub versioning_disabled: bool,
}
