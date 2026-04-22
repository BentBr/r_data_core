#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use base64::Engine as _;
use uuid::Uuid;

use r_data_core_core::outbox::{
    OutboxStatus, WORKFLOW_FETCH_ENQUEUE_KIND, WORKFLOW_FETCH_TOPIC, WORKFLOW_PUSH_ENQUEUE_KIND,
    WORKFLOW_PUSH_TOPIC,
};
use r_data_core_persistence::{OutboxMessageRecord, OutboxRepository};
use r_data_core_workflow::data::adapters::auth::{create_auth_provider, AuthConfig};
use r_data_core_workflow::data::adapters::destination::uri::UriDestination;
use r_data_core_workflow::data::adapters::destination::DataDestination;
use r_data_core_workflow::data::adapters::destination::{DestinationContext, HttpMethod};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;

use super::payload::{
    destination_method_name, encode_workflow_push_payload, WorkflowPushOutboxPayload,
};
use super::policy::{workflow_outbox_retry_at, OutboxRetryPolicy};
use super::support::{is_permanent_outbox_failure, parse_http_method, workflow_push_fingerprint};
use super::{
    validate_workflow_push_outbox_size, WORKFLOW_OUTBOX_MAX_ATTEMPTS, WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES,
};

/// Dispatcher component responsible for outbox record delivery and state transitions.
pub struct WorkflowOutboxDispatcher<'a> {
    queue: Option<&'a dyn JobQueue>,
    outbox_repo: &'a OutboxRepository,
    locked_by: Option<&'a str>,
    retry_policy: Option<&'a OutboxRetryPolicy>,
}

impl<'a> WorkflowOutboxDispatcher<'a> {
    #[must_use]
    pub const fn new(
        queue: Option<&'a dyn JobQueue>,
        outbox_repo: &'a OutboxRepository,
        locked_by: Option<&'a str>,
        retry_policy: Option<&'a OutboxRetryPolicy>,
    ) -> Self {
        Self {
            queue,
            outbox_repo,
            locked_by,
            retry_policy,
        }
    }

    /// Dispatch a fetch run to queue or transition to dead-letter/retry.
    ///
    /// # Errors
    /// Returns an error if queue interaction or state transition fails.
    pub async fn dispatch_fetch_run(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        outbox_uuid: Uuid,
        attempt_count: i32,
    ) -> r_data_core_core::error::Result<()> {
        let Some(queue) = self.queue else {
            self.outbox_repo
                .mark_dead_letter(
                    outbox_uuid,
                    "Workflow fetch outbox requires a queue",
                    self.locked_by,
                )
                .await?;
            return Ok(());
        };

        let job = FetchAndStageJob {
            workflow_id: workflow_uuid,
            trigger_id: Some(run_uuid),
        };

        match queue.enqueue_fetch(job).await {
            Ok(()) => {
                self.outbox_repo
                    .mark_delivered(outbox_uuid, self.locked_by)
                    .await?;
            }
            Err(e) => {
                if is_permanent_outbox_failure(&e) {
                    self.outbox_repo
                        .mark_dead_letter(outbox_uuid, &e.to_string(), self.locked_by)
                        .await?;
                    return Ok(());
                }

                let default_policy = OutboxRetryPolicy::default();
                let policy = self.retry_policy.map_or(&default_policy, |policy| policy);
                if attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                    self.outbox_repo
                        .mark_dead_letter(outbox_uuid, &e.to_string(), self.locked_by)
                        .await?;
                } else {
                    let next_available_at = workflow_outbox_retry_at(attempt_count, policy);
                    self.outbox_repo
                        .mark_retry(outbox_uuid, &e.to_string(), next_available_at, self.locked_by)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Dispatch any supported outbox record type.
    ///
    /// # Errors
    /// Returns an error if the underlying queue/push or database operation fails.
    pub async fn dispatch_record(
        &self,
        record: &OutboxMessageRecord,
    ) -> r_data_core_core::error::Result<()> {
        let locked_by = self.locked_by.or(record.locked_by.as_deref());

        if record.topic == WORKFLOW_FETCH_TOPIC && record.kind == WORKFLOW_FETCH_ENQUEUE_KIND {
            let job: FetchAndStageJob = match serde_json::from_value(record.payload.clone()) {
                Ok(job) => job,
                Err(e) => {
                    self.outbox_repo
                        .mark_dead_letter(
                            record.uuid,
                            &format!("Invalid workflow outbox payload: {e}"),
                            locked_by,
                        )
                        .await?;
                    return Ok(());
                }
            };

            let Some(run_uuid) = job.trigger_id else {
                self.outbox_repo
                    .mark_dead_letter(
                        record.uuid,
                        "Missing trigger_id in workflow outbox payload",
                        locked_by,
                    )
                    .await?;
                return Ok(());
            };

            return WorkflowOutboxDispatcher::new(
                self.queue,
                self.outbox_repo,
                locked_by,
                self.retry_policy,
            )
            .dispatch_fetch_run(
                job.workflow_id,
                run_uuid,
                record.uuid,
                record.attempt_count.saturating_add(1),
            )
            .await;
        }

        if record.topic == WORKFLOW_PUSH_TOPIC && record.kind == WORKFLOW_PUSH_ENQUEUE_KIND {
            return self.dispatch_push_record(record).await;
        }

        self.outbox_repo
            .mark_dead_letter(record.uuid, "Unsupported outbox message type", locked_by)
            .await?;
        Ok(())
    }

    /// Dispatch a workflow push outbox record to its HTTP destination.
    ///
    /// # Errors
    /// Returns an error if the HTTP push or database status update fails.
    #[allow(clippy::too_many_lines)]
    pub async fn dispatch_push_record(
        &self,
        record: &OutboxMessageRecord,
    ) -> r_data_core_core::error::Result<()> {
        let locked_by = self.locked_by.or(record.locked_by.as_deref());

        let payload: WorkflowPushOutboxPayload = match serde_json::from_value(record.payload.clone()) {
            Ok(payload) => payload,
            Err(e) => {
                self.outbox_repo
                    .mark_dead_letter(
                        record.uuid,
                        &format!("Invalid workflow push payload: {e}"),
                        locked_by,
                    )
                    .await?;
                return Ok(());
            }
        };

        if payload.destination_type.as_str() != "uri" {
            self.outbox_repo
                .mark_dead_letter(
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
            self.outbox_repo
                .mark_dead_letter(record.uuid, "URI destination is missing uri", locked_by)
                .await?;
            return Ok(());
        };

        let destination_auth = match payload.destination_auth {
            Some(auth) => match serde_json::from_value::<AuthConfig>(auth) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    self.outbox_repo
                        .mark_dead_letter(
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
                self.outbox_repo
                    .mark_dead_letter(
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
                self.outbox_repo
                    .mark_dead_letter(
                        record.uuid,
                        &format!("Invalid workflow push payload body: {e}"),
                        locked_by,
                    )
                    .await?;
                return Ok(());
            }
        };
        if data.len() > WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES {
            self.outbox_repo
                .mark_dead_letter(
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

/// Enqueue a workflow push delivery in the outbox.
///
/// # Errors
/// Returns an error if the payload is too large or the insert fails.
#[allow(clippy::too_many_arguments)]
pub async fn enqueue_workflow_push_outbox(
    outbox_repo: &OutboxRepository,
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    item_uuid: Uuid,
    destination_type: &str,
    destination_config: serde_json::Value,
    destination_auth: Option<serde_json::Value>,
    method: Option<HttpMethod>,
    format_type: &str,
    data_bytes: &[u8],
) -> r_data_core_core::error::Result<Uuid> {
    validate_workflow_push_outbox_size(data_bytes)?;
    let payload = WorkflowPushOutboxPayload {
        workflow_id: workflow_uuid,
        run_uuid,
        item_uuid,
        destination_type: destination_type.to_string(),
        destination_config: destination_config.clone(),
        destination_auth: destination_auth.clone(),
        method: method.map(destination_method_name).map(str::to_string),
        format_type: format_type.to_string(),
        data_base64: encode_workflow_push_payload(data_bytes),
    };
    let payload_value = serde_json::to_value(payload).map_err(|e| {
        r_data_core_core::error::Error::Validation(format!("Failed to encode push payload: {e}"))
    })?;
    let headers = serde_json::json!({
        "workflow_id": workflow_uuid,
        "run_uuid": run_uuid,
        "item_uuid": item_uuid,
        "topic": WORKFLOW_PUSH_TOPIC,
        "kind": WORKFLOW_PUSH_ENQUEUE_KIND,
        "destination_type": destination_type,
        "format_type": format_type,
    });
    let fingerprint = workflow_push_fingerprint(
        workflow_uuid,
        run_uuid,
        item_uuid,
        destination_type,
        &destination_config,
        method.as_ref(),
        data_bytes,
    );
    outbox_repo
        .insert_workflow_push_enqueue(
            workflow_uuid,
            run_uuid,
            item_uuid,
            payload_value,
            headers,
            &fingerprint,
        )
        .await
}

/// Convert a fetched outbox record into a simplified state view.
#[must_use]
pub fn outbox_status(record: &OutboxMessageRecord) -> OutboxStatus {
    record
        .status
        .parse::<OutboxStatus>()
        .unwrap_or(OutboxStatus::Pending)
}
