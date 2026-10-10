#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! Workflow API DTOs.
//!
//! `Deserialize` is derived on the response types as well as the request
//! types: API clients need to parse what the server sends. It has no effect on
//! the generated TypeScript.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

// Note: WorkflowKind is imported from the main crate's workflow module
// This is a temporary dependency until workflow is migrated to a crate
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowSummary {
    #[ts(type = "string")]
    pub uuid: Uuid,
    pub name: String,
    #[serde(rename = "kind")]
    pub kind: String, // Will be WorkflowKind once migrated
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    /// Indicates if this workflow has a from.api source type (accepts POST, cron disabled)
    #[serde(default)]
    pub has_api_endpoint: bool,
    #[serde(default)]
    pub versioning_disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowDetail {
    #[ts(type = "string")]
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "kind")]
    pub kind: String, // Will be WorkflowKind once migrated
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    #[ts(type = "unknown")]
    pub config: serde_json::Value,
    #[serde(default)]
    pub versioning_disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct CreateWorkflowResponse {
    #[ts(type = "string")]
    pub uuid: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowRunSummary {
    #[ts(type = "string")]
    pub uuid: Uuid,
    pub status: String,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    #[ts(type = "number | null")]
    pub processed_items: Option<i64>,
    #[ts(type = "number | null")]
    pub failed_items: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowRunLogDto {
    #[ts(type = "string")]
    pub uuid: Uuid,
    pub ts: String,
    pub level: String,
    pub message: String,
    #[ts(type = "unknown")]
    pub meta: Option<serde_json::Value>,
}

/// Multipart upload body for run-now file upload
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowRunUpload {
    /// CSV file to stage for this run
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowVersionMeta {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: time::OffsetDateTime,
    #[ts(type = "string | null")]
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WorkflowVersionPayload {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: time::OffsetDateTime,
    #[ts(type = "string | null")]
    pub created_by: Option<Uuid>,
    // The full `workflows` row as JSON, produced by `row_to_json`. The DSL
    // program is nested inside it under `consumer_config` or `provider_config`
    // depending on `kind` — there is no `config` column. Kept as a non-doc
    // comment: ts-rs emits `///` into the generated TypeScript, and this move
    // must leave the bindings byte-identical.
    #[ts(type = "unknown")]
    pub data: serde_json::Value,
}
