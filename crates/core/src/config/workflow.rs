#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Workflow engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Number of worker threads
    pub worker_threads: u32,

    /// Default timeout in seconds
    pub default_timeout: u64,

    /// Max concurrent workflows
    pub max_concurrent: u32,
}
