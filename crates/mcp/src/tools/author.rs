#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Creating and changing workflows.
//!
//! Every write goes through `validate_dsl` first, in the tool rather than in
//! the prompt. A model cannot persist a program that does not validate,
//! whatever it intends — which matters because the failure mode being guarded
//! against is not malice but confidence.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;

use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};

use super::discover::{error_result, json_result, parse_uuid};
use super::RdcTools;
use crate::client::ClientError;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateDslParams {
    /// The DSL steps array.
    pub steps: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkflowParams {
    /// A unique name for the workflow.
    pub name: String,
    /// "consumer" (pulls data in) or "provider" (serves data out).
    pub kind: String,
    /// The DSL steps array.
    pub steps: serde_json::Value,
    pub description: Option<String>,
    /// Defaults to false. Dry-run and test-run before enabling.
    pub enabled: Option<bool>,
    /// Cron expression. Check it with `preview_cron` first.
    pub schedule_cron: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateWorkflowParams {
    pub uuid: String,
    pub name: String,
    pub kind: String,
    /// The DSL steps array.
    pub steps: serde_json::Value,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub schedule_cron: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkflowUuidParams {
    pub uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VersionParams {
    pub uuid: String,
    pub version_number: i32,
}

#[tool_router(router = author_router, vis = "pub")]
impl RdcTools {
    /// Check a program without saving it.
    #[tool(
        name = "validate_dsl",
        description = "Check a DSL program for structural errors without saving it. \
                       Reports the exact JSON path of any problem and the legal values \
                       where the failure is an unknown variant. Validation cannot catch \
                       mapping or type errors that only appear at runtime — use \
                       test_workflow for those."
    )]
    pub async fn validate_dsl(
        &self,
        Parameters(params): Parameters<ValidateDslParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(
            match self
                .client
                .validate_dsl(&params.steps, &self.caller())
                .await
            {
                Ok(_) => json_result(&serde_json::json!({
                    "valid": true,
                    "note": "Structurally valid. Run test_workflow before saving: \
                             validation cannot see mapping or type errors."
                })),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Create a workflow, validating the program first.
    #[tool(
        name = "create_workflow",
        description = "Create a workflow. The DSL program is validated first and nothing \
                       is created if validation fails. New workflows are DISABLED by \
                       default: dry-run and test-run before enabling them with \
                       update_workflow."
    )]
    pub async fn create_workflow(
        &self,
        Parameters(params): Parameters<CreateWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.validate_or_reject(&params.steps).await {
            return Ok(error_result(&e));
        }

        let request = CreateWorkflowRequest {
            name: params.name,
            description: params.description,
            kind: params.kind,
            // Disabled unless explicitly asked for: a workflow that goes live
            // the instant it is created has skipped every check that would
            // have caught a mistake.
            enabled: params.enabled.unwrap_or(false),
            schedule_cron: params.schedule_cron,
            config: wrap_steps(&params.steps),
            versioning_disabled: false,
        };

        Ok(
            match self.client.create_workflow(&request, &self.caller()).await {
                Ok(created) => json_result(&serde_json::json!({
                    "uuid": created.uuid,
                    "enabled": request.enabled,
                    "next": "Run test_workflow, then run_workflow once, before enabling."
                })),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Update a workflow, validating the program first.
    #[tool(
        name = "update_workflow",
        description = "Update a workflow's program, name, schedule or enabled state. The \
                       DSL program is validated first and nothing is saved if validation \
                       fails. This is also how a workflow is enabled once tested."
    )]
    pub async fn update_workflow(
        &self,
        Parameters(params): Parameters<UpdateWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };
        if let Err(e) = self.validate_or_reject(&params.steps).await {
            return Ok(error_result(&e));
        }

        let request = UpdateWorkflowRequest {
            name: params.name,
            description: params.description,
            kind: params.kind,
            enabled: params.enabled.unwrap_or(false),
            schedule_cron: params.schedule_cron,
            config: wrap_steps(&params.steps),
            versioning_disabled: false,
        };

        Ok(
            match self
                .client
                .update_workflow(uuid, &request, &self.caller())
                .await
            {
                Ok(()) => json_result(&serde_json::json!({ "updated": true })),
                Err(e) => error_result(&e),
            },
        )
    }

    /// List a workflow's version history.
    #[tool(
        name = "list_workflow_versions",
        description = "List a workflow's versions, newest first, with who changed it and \
                       when. Use before restoring, to pick the right version."
    )]
    pub async fn list_workflow_versions(
        &self,
        Parameters(params): Parameters<WorkflowUuidParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };
        Ok(
            match self
                .client
                .list_workflow_versions(uuid, &self.caller())
                .await
            {
                Ok(versions) => json_result(&serde_json::to_value(versions).unwrap_or_default()),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Read one stored version.
    #[tool(
        name = "get_workflow_version",
        description = "Read a stored version of a workflow. The payload is the whole \
                       workflow row as it was, with the DSL program nested inside it."
    )]
    pub async fn get_workflow_version(
        &self,
        Parameters(params): Parameters<VersionParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };
        Ok(
            match self
                .client
                .get_workflow_version(uuid, params.version_number, &self.caller())
                .await
            {
                Ok(payload) => json_result(&serde_json::to_value(payload).unwrap_or_default()),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Roll a workflow's program back to an earlier version.
    #[tool(
        name = "restore_workflow_version",
        description = "Restore a workflow's program from an earlier version. This APPENDS \
                       a new version rather than rewinding history, so the change being \
                       rolled back stays in the record. Only the program is restored — \
                       name, enabled state and schedule are left as they are. Fails if \
                       the stored version is no longer a valid program."
    )]
    pub async fn restore_workflow_version(
        &self,
        Parameters(params): Parameters<VersionParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };
        Ok(
            match self
                .client
                .restore_workflow_version(uuid, params.version_number, &self.caller())
                .await
            {
                Ok(()) => json_result(&serde_json::json!({
                    "restored": true,
                    "note": "A new version was appended carrying the old program; \
                             history was not rewound."
                })),
                Err(e) => error_result(&e),
            },
        )
    }
}

impl RdcTools {
    /// Refuse to persist a program the server will not accept.
    ///
    /// Note how a validation failure actually arrives: the endpoint does not
    /// answer `{ valid: false }`. Success is 200; failure is a 422, which the
    /// client layer has already turned into a located `ClientError::Validation`.
    /// So the `?` here is the entire error path — there is no boolean to test.
    async fn validate_or_reject(&self, steps: &serde_json::Value) -> Result<(), ClientError> {
        self.client.validate_dsl(steps, &self.caller()).await?;
        Ok(())
    }
}

/// Wrap a bare steps array in the `{ "steps": [...] }` a workflow config needs.
///
/// Tools take the array because that is what a model naturally produces and
/// what `validate_dsl` accepts; the stored config wants the object. Accepting
/// either shape here means a model that passes the whole object does not get
/// a baffling error about a missing `steps` key.
fn wrap_steps(steps: &serde_json::Value) -> serde_json::Value {
    if steps.get("steps").is_some() {
        steps.clone()
    } else {
        serde_json::json!({ "steps": steps })
    }
}

#[cfg(test)]
mod tests;
