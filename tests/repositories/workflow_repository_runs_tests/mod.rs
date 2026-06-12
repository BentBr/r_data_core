#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
// Seed helpers propagate errors; doc comments would only add noise.
#![allow(clippy::missing_errors_doc)]

//! Integration tests for `workflow_repository/runs.rs`.
//!
//! Split into two sub-modules to stay under the 300-line file cap:
//! - `insert_and_status` — insert/queue/status/exists paths
//! - `paginated` — paginated list methods and log insertion

pub mod insert_and_status;
pub mod paginated;

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_workflow::data::requests::CreateWorkflowRequest;
use uuid::Uuid;

/// Seed a minimal consumer workflow.
pub async fn seed_workflow(
    repo: &WorkflowRepository,
    creator: Uuid,
    name: &str,
) -> Result<Uuid> {
    repo.create(
        &CreateWorkflowRequest {
            name: name.to_string(),
            description: None,
            kind: "consumer".to_string(),
            enabled: true,
            schedule_cron: None,
            config: serde_json::json!({}),
            versioning_disabled: false,
        },
        creator,
    )
    .await
}

/// Insert a queued run for `workflow_uuid`.
pub async fn seed_run(repo: &WorkflowRepository, workflow_uuid: Uuid) -> Result<Uuid> {
    repo.insert_run_queued(workflow_uuid, Uuid::now_v7()).await
}
