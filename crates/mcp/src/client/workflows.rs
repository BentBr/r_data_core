#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Workflow, run and version endpoints.
//!
//! Paths are the ones the router actually serves, not the ones the `OpenAPI`
//! annotations claim — the run-logs route in particular disagreed with its
//! annotation until recently, and the frontend was the tiebreaker.

use std::fmt::Write as _;

use serde_json::json;
use uuid::Uuid;

use r_data_core_core::dto::workflow::{
    CreateWorkflowResponse, WorkflowDetail, WorkflowRunLogDto, WorkflowRunSummary, WorkflowSummary,
    WorkflowVersionMeta, WorkflowVersionPayload,
};
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};

use super::{ClientError, Envelope, RdcClient};
use crate::auth::CallerContext;

impl RdcClient {
    /// # Errors
    /// Returns `ClientError` on transport failure or a non-2xx response.
    pub async fn list_workflows(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
        ctx: &CallerContext,
    ) -> Result<Vec<WorkflowSummary>, ClientError> {
        let mut path = "/admin/api/v1/workflows".to_string();
        let query: Vec<String> = limit
            .map(|l| format!("per_page={l}"))
            .into_iter()
            .chain(offset.map(|o| format!("offset={o}")))
            .collect();
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        self.get_json::<Envelope<Vec<WorkflowSummary>>>(&path, ctx)
            .await?
            .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn get_workflow(
        &self,
        uuid: Uuid,
        ctx: &CallerContext,
    ) -> Result<WorkflowDetail, ClientError> {
        self.get_json::<Envelope<WorkflowDetail>>(&format!("/admin/api/v1/workflows/{uuid}"), ctx)
            .await?
            .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn create_workflow(
        &self,
        request: &CreateWorkflowRequest,
        ctx: &CallerContext,
    ) -> Result<CreateWorkflowResponse, ClientError> {
        self.post_json::<_, Envelope<CreateWorkflowResponse>>(
            "/admin/api/v1/workflows",
            request,
            ctx,
        )
        .await?
        .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn update_workflow(
        &self,
        uuid: Uuid,
        request: &UpdateWorkflowRequest,
        ctx: &CallerContext,
    ) -> Result<(), ClientError> {
        // Update answers with a bare message and no data, so the envelope is
        // consumed for its status alone.
        let _: Envelope<serde_json::Value> = self
            .put_json(&format!("/admin/api/v1/workflows/{uuid}"), request, ctx)
            .await?;
        Ok(())
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn list_workflow_versions(
        &self,
        uuid: Uuid,
        ctx: &CallerContext,
    ) -> Result<Vec<WorkflowVersionMeta>, ClientError> {
        self.get_json::<Envelope<Vec<WorkflowVersionMeta>>>(
            &format!("/admin/api/v1/workflows/{uuid}/versions"),
            ctx,
        )
        .await?
        .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn get_workflow_version(
        &self,
        uuid: Uuid,
        version_number: i32,
        ctx: &CallerContext,
    ) -> Result<WorkflowVersionPayload, ClientError> {
        self.get_json::<Envelope<WorkflowVersionPayload>>(
            &format!("/admin/api/v1/workflows/{uuid}/versions/{version_number}"),
            ctx,
        )
        .await?
        .require_data()
    }

    /// Restore a previous version, appending a new one.
    ///
    /// A single call: restore semantics — which version becomes current, what
    /// the audit says, whether the payload is re-validated — belong to the
    /// server, not to a fetch-then-update dance in the client.
    ///
    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn restore_workflow_version(
        &self,
        uuid: Uuid,
        version_number: i32,
        ctx: &CallerContext,
    ) -> Result<(), ClientError> {
        let _: Envelope<serde_json::Value> = self
            .post_json(
                &format!("/admin/api/v1/workflows/{uuid}/versions/{version_number}/restore"),
                &json!({}),
                ctx,
            )
            .await?;
        Ok(())
    }

    /// Trigger a run. Returns immediately; the run proceeds asynchronously.
    ///
    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn run_workflow(
        &self,
        uuid: Uuid,
        input: Option<&serde_json::Value>,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, ClientError> {
        let body = input.cloned().unwrap_or_else(|| json!({}));
        let envelope: Envelope<serde_json::Value> = self
            .post_json(&format!("/admin/api/v1/workflows/{uuid}/run"), &body, ctx)
            .await?;
        Ok(envelope.data.unwrap_or(serde_json::Value::Null))
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn list_runs(
        &self,
        uuid: Option<Uuid>,
        limit: Option<u32>,
        ctx: &CallerContext,
    ) -> Result<Vec<WorkflowRunSummary>, ClientError> {
        let mut path = uuid.map_or_else(
            || "/admin/api/v1/workflows/runs".to_string(),
            |uuid| format!("/admin/api/v1/workflows/{uuid}/runs"),
        );
        if let Some(limit) = limit {
            let _ = write!(path, "?per_page={limit}");
        }
        self.get_json::<Envelope<Vec<WorkflowRunSummary>>>(&path, ctx)
            .await?
            .require_data()
    }

    /// Read a run's logs.
    ///
    /// Note the path: the handler's `OpenAPI` annotation claimed
    /// `/admin/api/v1/workflow-runs/...` while the Actix route is
    /// `/runs/{run_uuid}/logs` inside the `/workflows` scope. The annotation
    /// was the wrong one and has been corrected, but the route is what matters
    /// here.
    ///
    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn get_run_logs(
        &self,
        run_uuid: Uuid,
        limit: Option<u32>,
        ctx: &CallerContext,
    ) -> Result<Vec<WorkflowRunLogDto>, ClientError> {
        let mut path = format!("/admin/api/v1/workflows/runs/{run_uuid}/logs");
        if let Some(limit) = limit {
            let _ = write!(path, "?per_page={limit}");
        }
        self.get_json::<Envelope<Vec<WorkflowRunLogDto>>>(&path, ctx)
            .await?
            .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn preview_cron(
        &self,
        expression: &str,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, ClientError> {
        let encoded = urlencode(expression);
        self.get_json::<Envelope<serde_json::Value>>(
            &format!("/admin/api/v1/workflows/cron/preview?expression={encoded}"),
            ctx,
        )
        .await?
        .require_data()
    }

    /// # Errors
    /// As [`Self::list_workflows`].
    pub async fn permissions(&self, ctx: &CallerContext) -> Result<serde_json::Value, ClientError> {
        self.get_json::<Envelope<serde_json::Value>>("/admin/api/v1/auth/permissions", ctx)
            .await?
            .require_data()
    }
}

/// Percent-encode a query parameter value.
///
/// Cron expressions are full of characters a URL cannot carry raw — `*`, `/`,
/// spaces — and sending `0 2 * * *` unencoded produces a query the server
/// cannot parse.
fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests;
