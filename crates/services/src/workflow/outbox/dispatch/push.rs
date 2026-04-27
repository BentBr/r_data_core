use base64::Engine as _;

use r_data_core_workflow::data::adapters::auth::{create_auth_provider, AuthConfig};
use r_data_core_workflow::data::adapters::destination::uri::UriDestination;
use r_data_core_workflow::data::adapters::destination::DataDestination;
use r_data_core_workflow::data::adapters::destination::{DestinationContext, HttpMethod};

use super::super::payload::WorkflowPushOutboxPayload;
use super::super::policy::{workflow_outbox_retry_at, OutboxRetryPolicy};
use super::super::support::{is_permanent_outbox_failure, parse_http_method};
use super::super::{WORKFLOW_OUTBOX_MAX_ATTEMPTS, WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES};
use super::dispatcher::WorkflowOutboxDispatcher;

impl<'a> WorkflowOutboxDispatcher<'a> {
    /// Dispatch a workflow push outbox record to its HTTP destination.
    ///
    /// # Errors
    /// Returns an error if the HTTP push or database status update fails.
    #[allow(clippy::too_many_lines)]
    pub async fn dispatch_push_record(
        &self,
        record: &r_data_core_persistence::OutboxMessageRecord,
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

        let Some(uri) = payload
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

        let destination_auth = match payload.destination_auth {
            Some(auth) => match serde_json::from_value::<AuthConfig>(auth) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    self.mark_dead_letter_for_record(
                        record.uuid,
                        &format!("Invalid workflow push auth configuration: {e}"),
                        locked_by,
                    )
                    .await?;
                    return Ok(());
                }
            },
            None => None,
        };
        let auth_provider = match destination_auth
            .as_ref()
            .map(create_auth_provider)
            .transpose()
        {
            Ok(provider) => provider,
            Err(e) => {
                self.mark_dead_letter_for_record(
                    record.uuid,
                    &format!("Failed to create auth provider for workflow push: {e}"),
                    locked_by,
                )
                .await?;
                return Ok(());
            }
        };
        let method = payload
            .method
            .as_deref()
            .and_then(parse_http_method)
            .unwrap_or(HttpMethod::Post);
        let dest_ctx = DestinationContext {
            auth: auth_provider,
            method: Some(method),
            config: serde_json::json!({ "uri": uri }),
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
