#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Running programs, and finding out what happened.
//!
//! The distinction these tools draw is the one that makes the authoring loop
//! usable: `test_workflow` executes for real but persists nothing, so a model
//! can iterate freely; `run_workflow` has genuine side effects and says so in
//! the first sentence of its description.

use std::time::Duration;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use super::discover::{error_result, json_result, parse_uuid};
use super::RdcTools;
use crate::auth::CallerContext;
use crate::client::ClientError;

/// How long `run_workflow` waits before handing back a run UUID.
///
/// Deliberately below the ~60s tool-call timeout MCP clients typically impose.
/// A server-side wait that outlives the client's patience turns every slow run
/// into a client-side failure with no result at all, which is strictly worse
/// than returning early with a usable handle.
const DEFAULT_WAIT_SECS: u64 = 25;
/// Ceiling on the caller's requested wait, for the same reason.
const MAX_WAIT_SECS: u64 = 55;
/// Gap between status checks while waiting.
const POLL_INTERVAL: Duration = Duration::from_millis(1000);

// Enforced at compile time rather than by a test: a wait longer than the
// client's own tool-call timeout converts a slow run into a failure with no
// result, and that must not be possible to introduce accidentally.
const _: () = assert!(DEFAULT_WAIT_SECS < 60 && MAX_WAIT_SECS < 60);

/// Run statuses that will not change again.
///
/// From `RunStatus` in `crates/workflow/src/data/mod.rs`, which serializes
/// lowercase.
const TERMINAL_STATUSES: &[&str] = &["success", "failed", "cancelled"];

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestWorkflowParams {
    /// The DSL steps array to test. Give this or `uuid`.
    pub steps: Option<serde_json::Value>,
    /// Test a saved workflow's current program instead of supplying one.
    pub uuid: Option<String>,
    /// Sample input the first step reads.
    pub input: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunWorkflowParams {
    pub uuid: String,
    /// Payload for workflows whose first step reads from the API.
    pub input: Option<serde_json::Value>,
    /// Wait for the run to finish and return its logs. Defaults to true.
    pub wait: Option<bool>,
    /// Seconds to wait. Defaults to 25, capped at 55.
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRunsParams {
    /// Limit to one workflow. Omit for runs across all of them.
    pub uuid: Option<String>,
    /// Filter by status: queued, running, success, failed or cancelled.
    pub status: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunLogsParams {
    pub run_uuid: String,
    /// Filter by level, e.g. "error".
    pub level: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreviewCronParams {
    /// A cron expression, e.g. "0 2 * * *".
    pub expression: String,
    /// How many fire times to show. Defaults to the server's own default.
    pub count: Option<u32>,
}

#[tool_router(router = execute_router, vis = "pub")]
impl RdcTools {
    /// Execute a program against sample input, changing nothing.
    #[tool(
        name = "test_workflow",
        description = "Execute a DSL program against sample input WITHOUT persisting \
                       anything, and return a per-step trace of what each step produced. \
                       Entity reads and writes run against an in-memory overlay, so \
                       mappings and lookups are genuinely exercised; email and outbound \
                       pushes are never attempted. Accepts an unsaved program, so a draft \
                       can be tested before the workflow exists. Iterate here freely."
    )]
    pub async fn test_workflow(
        &self,
        Parameters(params): Parameters<TestWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let steps = match self.resolve_steps(&params).await {
            Ok(steps) => steps,
            Err(e) => return Ok(error_result(&e)),
        };

        Ok(
            match self
                .client
                .dry_run(&steps, &params.input, &self.caller())
                .await
            {
                Ok(response) => json_result(&serde_json::to_value(response).unwrap_or_default()),
                Err(e) => error_result(&e),
            },
        )
    }

    /// Execute a workflow for real.
    #[tool(
        name = "run_workflow",
        description = "Execute a workflow FOR REAL: entity writes, emails and outbound \
                       pushes all happen. Use test_workflow first. By default this waits \
                       for the run to finish and returns its status together with its \
                       logs. If it times out the run is still executing — poll with \
                       get_run_logs rather than running it again."
    )]
    pub async fn run_workflow(
        &self,
        Parameters(params): Parameters<RunWorkflowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match parse_uuid(&params.uuid, "uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };

        if let Err(e) = self
            .client
            .run_workflow(uuid, params.input.as_ref(), &self.caller())
            .await
        {
            return Ok(error_result(&e));
        }

        if !params.wait.unwrap_or(true) {
            return Ok(json_result(&serde_json::json!({
                "started": true,
                "next": "Poll list_runs for status, then get_run_logs."
            })));
        }

        let deadline = Duration::from_secs(
            params
                .timeout_secs
                .unwrap_or(DEFAULT_WAIT_SECS)
                .min(MAX_WAIT_SECS),
        );
        Ok(self.await_run(uuid, deadline).await)
    }

    /// List runs.
    #[tool(
        name = "list_runs",
        description = "List workflow runs with their status and item counts, newest \
                       first. Omit uuid to see runs across all workflows — useful for \
                       finding a recent failure."
    )]
    pub async fn list_runs(
        &self,
        Parameters(params): Parameters<ListRunsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let uuid = match params.uuid.as_deref().map(|raw| parse_uuid(raw, "uuid")) {
            Some(Ok(uuid)) => Some(uuid),
            Some(Err(e)) => return Ok(error_result(&e)),
            None => None,
        };

        Ok(
            match self
                .client
                .list_runs(uuid, params.limit, &self.caller())
                .await
            {
                Ok(runs) => {
                    // Filtered here because the endpoint has no status filter;
                    // silently ignoring the parameter would be worse.
                    let filtered: Vec<_> = runs
                        .into_iter()
                        .filter(|r| {
                            params
                                .status
                                .as_ref()
                                .is_none_or(|want| r.status.eq_ignore_ascii_case(want))
                        })
                        .collect();
                    json_result(&serde_json::json!({ "runs": filtered }))
                }
                Err(e) => error_result(&e),
            },
        )
    }

    /// Read a run's logs.
    #[tool(
        name = "get_run_logs",
        description = "Read a run's structured logs. The meta field usually names the \
                       failing step, which is where to start when debugging."
    )]
    pub async fn get_run_logs(
        &self,
        Parameters(params): Parameters<RunLogsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let run_uuid = match parse_uuid(&params.run_uuid, "run_uuid") {
            Ok(uuid) => uuid,
            Err(e) => return Ok(error_result(&e)),
        };

        Ok(
            match self
                .client
                .get_run_logs(run_uuid, params.limit, &self.caller())
                .await
            {
                Ok(logs) => {
                    let filtered: Vec<_> = logs
                        .into_iter()
                        .filter(|l| {
                            params
                                .level
                                .as_ref()
                                .is_none_or(|want| l.level.eq_ignore_ascii_case(want))
                        })
                        .collect();
                    json_result(&serde_json::json!({ "logs": filtered }))
                }
                Err(e) => error_result(&e),
            },
        )
    }

    /// Show when a cron expression would fire.
    #[tool(
        name = "preview_cron",
        description = "Show the next fire times for a cron expression. Check a schedule \
                       here before setting it on a workflow."
    )]
    pub async fn preview_cron(
        &self,
        Parameters(params): Parameters<PreviewCronParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(
            match self
                .client
                .preview_cron(&params.expression, &self.caller())
                .await
            {
                Ok(preview) => json_result(&preview),
                Err(e) => error_result(&e),
            },
        )
    }
}

impl RdcTools {
    /// The program to test: the one supplied, or a saved workflow's current one.
    async fn resolve_steps(
        &self,
        params: &TestWorkflowParams,
    ) -> Result<serde_json::Value, ClientError> {
        if let Some(steps) = &params.steps {
            return Ok(steps.clone());
        }
        let Some(raw_uuid) = &params.uuid else {
            return Err(ClientError::Validation {
                json_path: Some("steps".to_string()),
                message: "give either 'steps' (a program to test) or 'uuid' (a saved \
                          workflow whose program to test)"
                    .to_string(),
                legal_values: Vec::new(),
            });
        };
        let uuid = parse_uuid(raw_uuid, "uuid")?;
        let detail = self.client.get_workflow(uuid, &self.caller()).await?;
        // A stored config is { "steps": [...] }; the dry-run endpoint wants
        // the array.
        Ok(detail.config.get("steps").cloned().unwrap_or(detail.config))
    }

    /// Poll until the run reaches a terminal state or the deadline passes.
    async fn await_run(&self, uuid: Uuid, deadline: Duration) -> CallToolResult {
        let started = std::time::Instant::now();
        let ctx = CallerContext::default();

        loop {
            match self.client.list_runs(Some(uuid), Some(1), &ctx).await {
                Ok(runs) => {
                    if let Some(run) = runs.first() {
                        if TERMINAL_STATUSES
                            .iter()
                            .any(|s| run.status.eq_ignore_ascii_case(s))
                        {
                            return self.terminal_result(run).await;
                        }
                    }
                }
                Err(e) => return error_result(&e),
            }

            if started.elapsed() >= deadline {
                return json_result(&serde_json::json!({
                    "timed_out": true,
                    "note": "The run is still executing. Poll list_runs and then \
                             get_run_logs — do NOT run the workflow again, which would \
                             double its side effects."
                }));
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// Status and logs together, so the model does not need a second turn.
    async fn terminal_result(
        &self,
        run: &r_data_core_core::dto::workflow::WorkflowRunSummary,
    ) -> CallToolResult {
        let logs = self
            .client
            .get_run_logs(run.uuid, Some(100), &CallerContext::default())
            .await
            .unwrap_or_default();

        json_result(&serde_json::json!({
            "run_uuid": run.uuid,
            "status": run.status,
            "processed_items": run.processed_items,
            "failed_items": run.failed_items,
            "timed_out": false,
            "logs": logs,
        }))
    }
}

#[cfg(test)]
mod tests;
