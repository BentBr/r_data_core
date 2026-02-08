#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Workflow run logs retention configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunLogSettings {
    /// Whether workflow run logs pruning is enabled
    pub enabled: bool,
    /// Maximum number of runs to keep per workflow (None = unlimited)
    pub max_runs: Option<i32>,
    /// Maximum age in days for workflow runs (None = no age limit)
    pub max_age_days: Option<i32>,
}

impl Default for WorkflowRunLogSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_runs: None,
            max_age_days: Some(90),
        }
    }
}
