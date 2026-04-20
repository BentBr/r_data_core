#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPushOutboxPayload {
    pub workflow_id: Uuid,
    pub run_uuid: Uuid,
    pub item_uuid: Uuid,
    pub destination_type: String,
    pub destination_config: serde_json::Value,
    pub destination_auth: Option<serde_json::Value>,
    pub method: Option<String>,
    pub format_type: String,
    pub data_base64: String,
}

/// Maximum number of workflow outbox items to claim in one pass.
pub const WORKFLOW_OUTBOX_BATCH_SIZE: i64 = 50;

/// Duration after which a processing lease is considered stale.
pub const WORKFLOW_OUTBOX_STALE_LEASE_SECS: i64 = 300;

/// Maximum number of attempts before a workflow outbox entry is dead-lettered.
pub const WORKFLOW_OUTBOX_MAX_ATTEMPTS: i32 = 10;

/// Compute the next retry timestamp using a capped exponential backoff.
#[must_use]
pub fn workflow_outbox_retry_at(attempt_count: i32) -> OffsetDateTime {
    let exponent = attempt_count.saturating_sub(1).clamp(0, 6) as u32;
    let delay_secs = 2_i64.pow(exponent).min(300);
    OffsetDateTime::now_utc() + time::Duration::seconds(delay_secs)
}

/// Dispatch a single workflow fetch outbox record to Redis.
///
/// # Errors
/// Returns an error if the Redis enqueue or database status update fails.
pub async fn dispatch_workflow_fetch_job(
    queue: &dyn JobQueue,
    outbox_repo: &OutboxRepository,
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    outbox_uuid: Uuid,
    attempt_count: i32,
) -> r_data_core_core::error::Result<()> {
    let job = FetchAndStageJob {
        workflow_id: workflow_uuid,
        trigger_id: Some(run_uuid),
    };

    match queue.enqueue_fetch(job).await {
        Ok(()) => {
            outbox_repo.mark_delivered(outbox_uuid).await?;
        }
        Err(e) => {
            if attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                outbox_repo
                    .mark_dead_letter(outbox_uuid, &e.to_string())
                    .await?;
            } else {
                let next_available_at = workflow_outbox_retry_at(attempt_count);
                outbox_repo
                    .mark_retry(outbox_uuid, &e.to_string(), next_available_at)
                    .await?;
            }
        }
    }

    Ok(())
}

/// Enqueue a workflow push delivery in the outbox.
///
/// # Errors
/// Returns an error if the insert fails.
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
    let payload = WorkflowPushOutboxPayload {
        workflow_id: workflow_uuid,
        run_uuid,
        item_uuid,
        destination_type: destination_type.to_string(),
        destination_config: destination_config.clone(),
        destination_auth: destination_auth.clone(),
        method: method.map(|value| http_method_name(value).to_string()),
        format_type: format_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(data_bytes),
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

/// Dispatch a single workflow fetch outbox record to Redis.
///
/// # Errors
/// Returns an error if the Redis enqueue or database status update fails.
pub async fn dispatch_workflow_outbox(
    queue: Option<&dyn JobQueue>,
    outbox_repo: &OutboxRepository,
    record: &OutboxMessageRecord,
) -> r_data_core_core::error::Result<()> {
    if record.topic == WORKFLOW_FETCH_TOPIC && record.kind == WORKFLOW_FETCH_ENQUEUE_KIND {
        let job: FetchAndStageJob = match serde_json::from_value(record.payload.clone()) {
            Ok(job) => job,
            Err(e) => {
                outbox_repo
                    .mark_dead_letter(
                        record.uuid,
                        &format!("Invalid workflow outbox payload: {e}"),
                    )
                    .await?;
                return Ok(());
            }
        };

        let Some(run_uuid) = job.trigger_id else {
            outbox_repo
                .mark_dead_letter(record.uuid, "Missing trigger_id in workflow outbox payload")
                .await?;
            return Ok(());
        };

        let Some(queue) = queue else {
            outbox_repo
                .mark_dead_letter(record.uuid, "Workflow fetch outbox requires a queue")
                .await?;
            return Ok(());
        };

        dispatch_workflow_fetch_job(
            queue,
            outbox_repo,
            job.workflow_id,
            run_uuid,
            record.uuid,
            record.attempt_count,
        )
        .await?;
        return Ok(());
    }

    if record.topic == WORKFLOW_PUSH_TOPIC && record.kind == WORKFLOW_PUSH_ENQUEUE_KIND {
        return dispatch_workflow_push_outbox(outbox_repo, record).await;
    }

    outbox_repo
        .mark_dead_letter(record.uuid, "Unsupported outbox message type")
        .await?;
    Ok(())
}

/// Dispatch a single workflow push outbox record to its HTTP destination.
///
/// # Errors
/// Returns an error if the HTTP push or database status update fails.
pub async fn dispatch_workflow_push_outbox(
    outbox_repo: &OutboxRepository,
    record: &OutboxMessageRecord,
) -> r_data_core_core::error::Result<()> {
    let payload: WorkflowPushOutboxPayload = match serde_json::from_value(record.payload.clone()) {
        Ok(payload) => payload,
        Err(e) => {
            outbox_repo
                .mark_dead_letter(record.uuid, &format!("Invalid workflow push payload: {e}"))
                .await?;
            return Ok(());
        }
    };

    if payload.destination_type.as_str() != "uri" {
        outbox_repo
            .mark_dead_letter(
                record.uuid,
                &format!(
                    "Unsupported workflow push destination type: {}",
                    payload.destination_type
                ),
            )
            .await?;
        return Ok(());
    }

    let Some(uri) = payload
        .destination_config
        .get("uri")
        .and_then(|value| value.as_str())
    else {
        outbox_repo
            .mark_dead_letter(record.uuid, "URI destination is missing uri")
            .await?;
        return Ok(());
    };

    let destination_auth = match payload.destination_auth {
        Some(auth) => Some(serde_json::from_value::<AuthConfig>(auth).map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Invalid workflow push auth configuration: {e}"
            ))
        })?),
        None => None,
    };
    let auth_provider = destination_auth
        .as_ref()
        .map(create_auth_provider)
        .transpose()
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!(
                "Failed to create auth provider for workflow push: {e}"
            ))
        })?;
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
            outbox_repo
                .mark_dead_letter(
                    record.uuid,
                    &format!("Invalid workflow push payload body: {e}"),
                )
                .await?;
            return Ok(());
        }
    };

    let result = destination.push(&dest_ctx, bytes::Bytes::from(data)).await;
    match result {
        Ok(()) => {
            outbox_repo.mark_delivered(record.uuid).await?;
        }
        Err(e) => {
            if record.attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                outbox_repo
                    .mark_dead_letter(record.uuid, &e.to_string())
                    .await?;
            } else {
                let next_available_at = workflow_outbox_retry_at(record.attempt_count);
                outbox_repo
                    .mark_retry(record.uuid, &e.to_string(), next_available_at)
                    .await?;
            }
        }
    }

    Ok(())
}

/// Claim and dispatch due workflow fetch outbox rows.
///
/// # Errors
/// Returns an error if claiming or dispatching fails.
pub async fn claim_and_dispatch_workflow_outbox(
    queue: &dyn JobQueue,
    outbox_repo: &OutboxRepository,
    worker_id: &str,
    batch_size: i64,
) -> r_data_core_core::error::Result<usize> {
    let stale_before =
        OffsetDateTime::now_utc() - time::Duration::seconds(WORKFLOW_OUTBOX_STALE_LEASE_SECS);
    let _ = outbox_repo.requeue_stale_processing(stale_before).await?;

    let records = outbox_repo.claim_due(batch_size, worker_id).await?;
    let mut dispatched = 0usize;
    for record in records {
        dispatch_workflow_outbox(Some(queue), outbox_repo, &record).await?;
        dispatched = dispatched.saturating_add(1);
    }

    Ok(dispatched)
}

/// Convert a fetched outbox record into a simplified state view.
#[must_use]
pub fn outbox_status(record: &OutboxMessageRecord) -> OutboxStatus {
    record
        .status
        .parse::<OutboxStatus>()
        .unwrap_or(OutboxStatus::Pending)
}

fn parse_http_method(value: &str) -> Option<HttpMethod> {
    match value {
        "GET" => Some(HttpMethod::Get),
        "POST" => Some(HttpMethod::Post),
        "PUT" => Some(HttpMethod::Put),
        "PATCH" => Some(HttpMethod::Patch),
        "DELETE" => Some(HttpMethod::Delete),
        "HEAD" => Some(HttpMethod::Head),
        "OPTIONS" => Some(HttpMethod::Options),
        _ => None,
    }
}

fn workflow_push_fingerprint(
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    item_uuid: Uuid,
    destination_type: &str,
    destination_config: &serde_json::Value,
    method: Option<&HttpMethod>,
    data_bytes: &[u8],
) -> String {
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(workflow_uuid.as_bytes());
    hasher.update(run_uuid.as_bytes());
    hasher.update(item_uuid.as_bytes());
    hasher.update(destination_type.as_bytes());
    hasher.update(destination_config.to_string().as_bytes());
    if let Some(method) = method {
        hasher.update(http_method_name(*method).as_bytes());
    }
    hasher.update(data_bytes);
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

fn http_method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}
