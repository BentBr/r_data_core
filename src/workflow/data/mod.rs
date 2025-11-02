pub mod adapters;
pub mod job_queue;
pub mod jobs;
pub mod repository;
pub mod repository_trait;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowKind {
    Consumer,
    Provider,
}

impl Display for WorkflowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowKind::Consumer => write!(f, "consumer"),
            WorkflowKind::Provider => write!(f, "provider"),
        }
    }
}

impl FromStr for WorkflowKind {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "consumer" => Ok(WorkflowKind::Consumer),
            "provider" => Ok(WorkflowKind::Provider),
            _ => Err("invalid workflow kind"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub adapter: String,
    pub source: serde_json::Value,
    pub format: serde_json::Value,
    pub target: serde_json::Value,
    pub mapping: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub adapter: String,
    pub query: serde_json::Value,
    pub format: serde_json::Value,
    pub mapping: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: WorkflowKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub consumer_config: Option<serde_json::Value>,
    pub provider_config: Option<serde_json::Value>,
}
