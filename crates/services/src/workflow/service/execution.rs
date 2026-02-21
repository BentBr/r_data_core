use crate::workflow::item_processing::{
    execute_pipeline_inline, process_single_item, WorkflowItemContext,
};
use crate::workflow::transform_execution::JwtConfig;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::WorkflowService;

impl WorkflowService {
    /// Process staged raw items for a run using the workflow DSL
    ///
    /// # Errors
    /// Returns an error if workflow not found, DSL validation fails, or processing fails
    pub async fn process_staged_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<(i64, i64)> {
        let wf = self.repo.get_by_uuid(workflow_uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound("Workflow not found".to_string())
        })?;

        // Build DSL program from config; require presence and validation
        let program = match r_data_core_workflow::dsl::DslProgram::from_config(&wf.config) {
            Ok(p) => {
                if let Err(e) = p.validate() {
                    return self
                        .fail_entire_run_due_to_invalid_dsl(run_uuid, e.to_string())
                        .await;
                }
                p
            }
            _ => {
                return self
                    .fail_entire_run_due_to_invalid_dsl(
                        run_uuid,
                        "Missing or invalid DSL configuration".to_string(),
                    )
                    .await;
            }
        };

        let mut processed = 0_i64;
        let mut failed = 0_i64;
        loop {
            let items = self.repo.fetch_staged_raw_items(run_uuid, 200).await?;
            if items.is_empty() {
                break;
            }
            let jwt = JwtConfig {
                secret: self.jwt_secret.as_deref(),
                expiration: self.jwt_expiration,
            };
            let ctx = WorkflowItemContext {
                dynamic_entity_service: self.dynamic_entity_service.as_deref(),
                repo: &self.repo,
                jwt: &jwt,
                versioning_disabled: wf.versioning_disabled,
            };
            for (item_uuid, payload) in items {
                let success =
                    process_single_item(&program, &payload, item_uuid, run_uuid, &ctx).await?;
                if success {
                    processed += 1;
                } else {
                    failed += 1;
                }
            }
        }
        Ok((processed, failed))
    }

    /// Process a single payload inline (synchronously) through the workflow pipeline.
    ///
    /// Used for auth workflows that must return a response directly (e.g. login).
    /// Creates a run for logging but does not stage items. Returns the first
    /// `ToDef::Format` output.
    ///
    /// # Errors
    /// Returns an error if workflow not found, DSL invalid, or pipeline execution fails
    pub async fn process_payload_inline(
        &self,
        workflow_uuid: Uuid,
        payload: &JsonValue,
    ) -> r_data_core_core::error::Result<JsonValue> {
        let wf = self.repo.get_by_uuid(workflow_uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound("Workflow not found".to_string())
        })?;

        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&wf.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid DSL configuration: {e}"
                ))
            })?;
        program.validate().map_err(|e| {
            r_data_core_core::error::Error::Validation(format!("DSL validation failed: {e}"))
        })?;

        // Create a run for logging/history
        let run_uuid = self
            .repo
            .insert_run_queued(workflow_uuid, Uuid::now_v7())
            .await?;
        let _ = self.repo.mark_run_running(run_uuid).await;

        let jwt = JwtConfig {
            secret: self.jwt_secret.as_deref(),
            expiration: self.jwt_expiration,
        };
        let ctx = WorkflowItemContext {
            dynamic_entity_service: self.dynamic_entity_service.as_deref(),
            repo: &self.repo,
            jwt: &jwt,
            versioning_disabled: wf.versioning_disabled,
        };

        match execute_pipeline_inline(&program, payload, run_uuid, &ctx).await {
            Ok(outputs) => {
                let _ = self.repo.mark_run_success(run_uuid, 1, 0).await;
                // Return the first Format output value
                for (to_def, value) in outputs {
                    if matches!(to_def, r_data_core_workflow::dsl::ToDef::Format { .. }) {
                        return Ok(value);
                    }
                }
                Err(r_data_core_core::error::Error::Validation(
                    "No format output produced".to_string(),
                ))
            }
            Err(e) => {
                let _ = self.repo.mark_run_failure(run_uuid, &e.to_string()).await;
                Err(e)
            }
        }
    }

    async fn fail_entire_run_due_to_invalid_dsl(
        &self,
        run_uuid: Uuid,
        message: String,
    ) -> r_data_core_core::error::Result<(i64, i64)> {
        let _ = self
            .repo
            .insert_run_log(run_uuid, "error", &message, None)
            .await;
        // Mark all queued items as failed to prevent re-processing loops
        loop {
            let items = self.repo.fetch_staged_raw_items(run_uuid, 500).await?;
            if items.is_empty() {
                break;
            }
            for (item_uuid, _payload) in items {
                let _ = self
                    .repo
                    .set_raw_item_status(item_uuid, "failed", Some("Invalid DSL"))
                    .await;
            }
        }
        let _ = self.repo.mark_run_failure(run_uuid, &message).await;
        Err(r_data_core_core::error::Error::Validation(message))
    }
}
