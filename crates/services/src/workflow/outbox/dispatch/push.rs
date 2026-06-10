use base64::Engine as _;

use r_data_core_workflow::data::adapters::auth::create_auth_provider;
use r_data_core_workflow::data::adapters::destination::uri::UriDestination;
use r_data_core_workflow::data::adapters::destination::DataDestination;
use r_data_core_workflow::data::adapters::destination::{DestinationContext, HttpMethod};
use r_data_core_workflow::dsl::{DslProgram, OutputMode, ToDef};

use super::super::payload::WorkflowPushOutboxPayload;
use super::super::policy::{workflow_outbox_retry_at, OutboxRetryPolicy};
use super::super::support::{is_permanent_outbox_failure, parse_http_method};
use super::super::{WORKFLOW_OUTBOX_MAX_ATTEMPTS, WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES};
use super::dispatcher::WorkflowOutboxDispatcher;

impl WorkflowOutboxDispatcher<'_> {
    /// Dispatch a workflow push outbox record to its HTTP destination.
    ///
    /// # Errors
    /// Returns an error if the HTTP push or database status update fails.
    #[allow(clippy::too_many_lines)]
    pub async fn dispatch_push_record(
        &self,
        record: &r_data_core_core::outbox::OutboxMessage,
    ) -> r_data_core_core::error::Result<()> {
        let locked_by = self.locked_by.or(record.locked_by.as_deref());

        let payload: WorkflowPushOutboxPayload =
            match serde_json::from_value(record.payload.clone()) {
                Ok(payload) => payload,
                Err(e) => {
                    self.mark_dead_letter_for_record(
                        record.uuid,
                        &format!("Invalid workflow push payload: {e}"),
                        locked_by,
                    )
                    .await?;
                    return Ok(());
                }
            };

        if payload.destination_type.as_str() != "uri" {
            self.mark_dead_letter_for_record(
                record.uuid,
                &format!(
                    "Unsupported workflow push destination type: {}",
                    payload.destination_type
                ),
                locked_by,
            )
            .await?;
            return Ok(());
        }

        let Some(_uri) = payload
            .destination_config
            .get("uri")
            .and_then(|value| value.as_str())
        else {
            self.mark_dead_letter_for_record(
                record.uuid,
                "URI destination is missing uri",
                locked_by,
            )
            .await?;
            return Ok(());
        };

        if payload.destination_auth.is_some() {
            self.mark_dead_letter_for_record(
                record.uuid,
                "Workflow push outbox does not support persisted destination auth",
                locked_by,
            )
            .await?;
            return Ok(());
        }
        let auth_provider = if payload.auth_required {
            let Some(workflow_repo) = self.workflow_repo else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth requires workflow repository access",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            let Some(step_index) = payload.destination_step_index else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth requires destination step index",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            let Some(workflow) = workflow_repo.get_by_uuid(payload.workflow_id).await? else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth workflow not found",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            let program = match DslProgram::from_config(&workflow.config) {
                Ok(program) => program,
                Err(e) => {
                    self.mark_dead_letter_for_record(
                        record.uuid,
                        &format!("Invalid workflow config for push auth resolution: {e}"),
                        locked_by,
                    )
                    .await?;
                    return Ok(());
                }
            };
            let Some(step) = program.steps.get(step_index) else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth step not found",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            let ToDef::Format {
                output:
                    OutputMode::Push {
                        destination,
                        method: _,
                    },
                ..
            } = &step.to
            else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth step is not a push output",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            let Some(auth_config) = destination.auth.as_ref() else {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    "Workflow push outbox auth config not found",
                    locked_by,
                )
                .await?;
                return Ok(());
            };
            match create_auth_provider(auth_config) {
                Ok(provider) => Some(provider),
                Err(e) => {
                    self.mark_dead_letter_for_record(
                        record.uuid,
                        &format!("Failed to create workflow push auth provider: {e}"),
                        locked_by,
                    )
                    .await?;
                    return Ok(());
                }
            }
        } else {
            None
        };
        let method = payload
            .method
            .as_deref()
            .and_then(parse_http_method)
            .unwrap_or(HttpMethod::Post);
        let dest_ctx = DestinationContext {
            auth: auth_provider,
            method: Some(method),
            config: payload.destination_config.clone(),
        };
        let destination = UriDestination::new();
        let data = match base64::engine::general_purpose::STANDARD.decode(payload.data_base64) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    &format!("Invalid workflow push payload body: {e}"),
                    locked_by,
                )
                .await?;
                return Ok(());
            }
        };
        if data.len() > WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES {
            self.mark_dead_letter_for_record(
                record.uuid,
                &format!(
                    "Workflow push payload body exceeds the maximum size of {WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES} bytes",
                ),
                locked_by,
            )
            .await?;
            return Ok(());
        }
        let result = destination.push(&dest_ctx, bytes::Bytes::from(data)).await;
        match result {
            Ok(()) => {
                self.outbox_repo
                    .mark_delivered(record.uuid, locked_by)
                    .await?;
            }
            Err(e) => {
                let default_policy = OutboxRetryPolicy::default();
                let policy = self.retry_policy.map_or(&default_policy, |policy| policy);
                let next_attempt_count = record.attempt_count.saturating_add(1);
                if next_attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                    self.outbox_repo
                        .mark_dead_letter(record.uuid, &e.to_string(), locked_by)
                        .await?;
                } else {
                    if is_permanent_outbox_failure(&e) {
                        self.outbox_repo
                            .mark_dead_letter(record.uuid, &e.to_string(), locked_by)
                            .await?;
                        return Ok(());
                    }
                    let next_available_at = workflow_outbox_retry_at(next_attempt_count, policy);
                    self.outbox_repo
                        .mark_retry(record.uuid, &e.to_string(), next_available_at, locked_by)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
