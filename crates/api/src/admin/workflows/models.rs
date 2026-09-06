#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! Workflow DTOs.
//!
//! The definitions live in `r_data_core_core::dto::workflow` so that API
//! clients can share them without linking actix-web. They are re-exported
//! here to keep call sites and the `OpenAPI` registration unchanged.

pub use r_data_core_core::dto::workflow::{
    CreateWorkflowResponse, WorkflowDetail, WorkflowRunLogDto, WorkflowRunSummary,
    WorkflowRunUpload, WorkflowSummary, WorkflowVersionMeta, WorkflowVersionPayload,
};

// Request types stay sourced from the workflow crate: `core` sits below
// `workflow` in the dependency graph and may not import it.
pub use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
