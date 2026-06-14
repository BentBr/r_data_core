#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
// Test seed helpers don't document error conditions.
#![allow(clippy::missing_errors_doc)]

//! Integration tests for `workflow_repository/crud.rs`.
//!
//! Split into two submodules to stay under the 300-line file cap:
//! - `basic` — create, get, count, update, delete
//! - `list_and_scheduled` — `list_all`, `list_paginated`, `list_scheduled_consumers`

pub mod basic;
pub mod list_and_scheduled;

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_workflow::data::requests::CreateWorkflowRequest;
use uuid::Uuid;

/// Seed a minimal workflow (no cron, no description).
///
/// # Errors
/// Propagates any database error from `WorkflowRepository::create`.
pub async fn seed(
    repo: &WorkflowRepository,
    creator: Uuid,
    name: &str,
    kind: &str,
) -> Result<Uuid> {
    repo.create(
        &CreateWorkflowRequest {
            name: name.to_string(),
            description: None,
            kind: kind.to_string(),
            enabled: true,
            schedule_cron: None,
            config: serde_json::json!({}),
            versioning_disabled: false,
        },
        creator,
    )
    .await
}

/// Seed a consumer workflow with a cron expression.
pub async fn seed_with_cron(
    repo: &WorkflowRepository,
    creator: Uuid,
    name: &str,
    config: serde_json::Value,
    enabled: bool,
) -> Result<Uuid> {
    repo.create(
        &CreateWorkflowRequest {
            name: name.to_string(),
            description: None,
            kind: "consumer".to_string(),
            enabled,
            schedule_cron: Some("0 * * * *".to_string()),
            config,
            versioning_disabled: false,
        },
        creator,
    )
    .await
}
