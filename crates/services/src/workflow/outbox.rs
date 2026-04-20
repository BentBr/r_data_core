#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Error;
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

/// Maximum raw payload size for workflow push outbox entries.
///
/// The payload is stored base64-encoded in `PostgreSQL`, so the effective row size is larger.
pub const WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES: usize = 256 * 1024;

/// Duration after which a processing lease is considered stale.
pub const WORKFLOW_OUTBOX_STALE_LEASE_SECS: i64 = 300;

/// Maximum number of attempts before a workflow outbox entry is dead-lettered.
pub const WORKFLOW_OUTBOX_MAX_ATTEMPTS: i32 = 10;

/// Retry policy for workflow outbox entries.
#[derive(Debug, Clone, Copy)]
pub struct OutboxRetryPolicy {
    pub base_delay_secs: i64,
    pub multiplier: u64,
    pub max_delay_secs: i64,
}

impl Default for OutboxRetryPolicy {
    fn default() -> Self {
        Self {
            base_delay_secs: 1,
            multiplier: 2,
            max_delay_secs: 300,
        }
    }
}

impl OutboxRetryPolicy {
    #[must_use]
    pub const fn new(base_delay_secs: i64, multiplier: u64, max_delay_secs: i64) -> Self {
        Self {
            base_delay_secs,
            multiplier,
            max_delay_secs,
        }
    }
}

/// Compute the delay in seconds for the next workflow outbox retry.
#[must_use]
pub fn workflow_outbox_retry_delay_secs(attempt_count: i32, policy: &OutboxRetryPolicy) -> i64 {
    let exponent = u32::try_from(attempt_count.saturating_sub(1).clamp(0, 31)).unwrap_or(0);
    let multiplier = i128::from(policy.multiplier.max(1));
    let cap = i128::from(policy.max_delay_secs.max(1));
    let mut delay_secs = i128::from(policy.base_delay_secs.max(1));

    for _ in 0..exponent {
        delay_secs = delay_secs.saturating_mul(multiplier).min(cap);
        if delay_secs >= cap {
            return i64::try_from(cap).unwrap_or(i64::MAX);
        }
    }

    i64::try_from(delay_secs.min(cap)).unwrap_or(i64::MAX)
}

/// Compute the next retry timestamp using a capped exponential backoff.
#[must_use]
pub fn workflow_outbox_retry_at(attempt_count: i32, policy: &OutboxRetryPolicy) -> OffsetDateTime {
    OffsetDateTime::now_utc()
        + time::Duration::seconds(workflow_outbox_retry_delay_secs(attempt_count, policy))
}

/// Dispatch a single workflow fetch outbox record to Redis.
///
/// # Errors
/// Returns an error if the Redis enqueue or database status update fails.
#[allow(clippy::too_many_arguments)]
pub async fn dispatch_workflow_fetch_job(
    queue: &dyn JobQueue,
    outbox_repo: &OutboxRepository,
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    outbox_uuid: Uuid,
    attempt_count: i32,
    locked_by: Option<&str>,
    retry_policy: Option<&OutboxRetryPolicy>,
) -> r_data_core_core::error::Result<()> {
    let job = FetchAndStageJob {
        workflow_id: workflow_uuid,
        trigger_id: Some(run_uuid),
    };

    match queue.enqueue_fetch(job).await {
        Ok(()) => {
            outbox_repo.mark_delivered(outbox_uuid, locked_by).await?;
        }
        Err(e) => {
            if is_permanent_outbox_failure(&e) {
                outbox_repo
                    .mark_dead_letter(outbox_uuid, &e.to_string(), locked_by)
                    .await?;
                return Ok(());
            }

            let default_policy = OutboxRetryPolicy::default();
            let policy = retry_policy.map_or(&default_policy, |policy| policy);
            if attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                outbox_repo
                    .mark_dead_letter(outbox_uuid, &e.to_string(), locked_by)
                    .await?;
            } else {
                let next_available_at = workflow_outbox_retry_at(attempt_count, policy);
                outbox_repo
                    .mark_retry(outbox_uuid, &e.to_string(), next_available_at, locked_by)
                    .await?;
            }
        }
    }

    Ok(())
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
    locked_by: Option<&str>,
    retry_policy: Option<&OutboxRetryPolicy>,
) -> r_data_core_core::error::Result<()> {
    let locked_by = locked_by.or(record.locked_by.as_deref());

    if record.topic == WORKFLOW_FETCH_TOPIC && record.kind == WORKFLOW_FETCH_ENQUEUE_KIND {
        let job: FetchAndStageJob = match serde_json::from_value(record.payload.clone()) {
            Ok(job) => job,
            Err(e) => {
                outbox_repo
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
            outbox_repo
                .mark_dead_letter(
                    record.uuid,
                    "Missing trigger_id in workflow outbox payload",
                    locked_by,
                )
                .await?;
            return Ok(());
        };

        let Some(queue) = queue else {
            outbox_repo
                .mark_dead_letter(
                    record.uuid,
                    "Workflow fetch outbox requires a queue",
                    locked_by,
                )
                .await?;
            return Ok(());
        };

        dispatch_workflow_fetch_job(
            queue,
            outbox_repo,
            job.workflow_id,
            run_uuid,
            record.uuid,
            record.attempt_count.saturating_add(1),
            locked_by,
            retry_policy,
        )
        .await?;
        return Ok(());
    }

    if record.topic == WORKFLOW_PUSH_TOPIC && record.kind == WORKFLOW_PUSH_ENQUEUE_KIND {
        return dispatch_workflow_push_outbox(outbox_repo, record, locked_by, retry_policy).await;
    }

    outbox_repo
        .mark_dead_letter(record.uuid, "Unsupported outbox message type", locked_by)
        .await?;
    Ok(())
}

/// Dispatch a single workflow push outbox record to its HTTP destination.
///
/// # Errors
/// Returns an error if the HTTP push or database status update fails.
#[allow(clippy::too_many_lines)]
pub async fn dispatch_workflow_push_outbox(
    outbox_repo: &OutboxRepository,
    record: &OutboxMessageRecord,
    locked_by: Option<&str>,
    retry_policy: Option<&OutboxRetryPolicy>,
) -> r_data_core_core::error::Result<()> {
    let locked_by = locked_by.or(record.locked_by.as_deref());

    let payload: WorkflowPushOutboxPayload = match serde_json::from_value(record.payload.clone()) {
        Ok(payload) => payload,
        Err(e) => {
            outbox_repo
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
        outbox_repo
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
        outbox_repo
            .mark_dead_letter(record.uuid, "URI destination is missing uri", locked_by)
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
                    locked_by,
                )
                .await?;
            return Ok(());
        }
    };
    if data.len() > WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES {
        outbox_repo
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
            outbox_repo.mark_delivered(record.uuid, locked_by).await?;
        }
        Err(e) => {
            let default_policy = OutboxRetryPolicy::default();
            let policy = retry_policy.map_or(&default_policy, |policy| policy);
            let next_attempt_count = record.attempt_count.saturating_add(1);
            if next_attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                outbox_repo
                    .mark_dead_letter(record.uuid, &e.to_string(), locked_by)
                    .await?;
            } else {
                if is_permanent_outbox_failure(&e) {
                    outbox_repo
                        .mark_dead_letter(record.uuid, &e.to_string(), locked_by)
                        .await?;
                    return Ok(());
                }
                let next_available_at = workflow_outbox_retry_at(next_attempt_count, policy);
                outbox_repo
                    .mark_retry(record.uuid, &e.to_string(), next_available_at, locked_by)
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
    claim_and_dispatch_workflow_outbox_with_stale_lease(
        queue,
        outbox_repo,
        worker_id,
        batch_size,
        WORKFLOW_OUTBOX_STALE_LEASE_SECS,
        None,
    )
    .await
}

/// Claim and dispatch due workflow fetch outbox rows with a configurable stale lease.
///
/// # Errors
/// Returns an error if claiming or dispatching fails.
pub async fn claim_and_dispatch_workflow_outbox_with_stale_lease(
    queue: &dyn JobQueue,
    outbox_repo: &OutboxRepository,
    worker_id: &str,
    batch_size: i64,
    stale_lease_secs: i64,
    retry_policy: Option<&OutboxRetryPolicy>,
) -> r_data_core_core::error::Result<usize> {
    let stale_before = OffsetDateTime::now_utc() - time::Duration::seconds(stale_lease_secs);
    let _ = outbox_repo.requeue_stale_processing(stale_before).await?;

    let records = outbox_repo.claim_due(batch_size, worker_id).await?;
    let mut dispatched = 0usize;
    for record in records {
        dispatch_workflow_outbox(
            Some(queue),
            outbox_repo,
            &record,
            Some(worker_id),
            retry_policy,
        )
        .await?;
        dispatched = dispatched.saturating_add(1);
    }

    Ok(dispatched)
}

const fn is_permanent_outbox_failure(error: &Error) -> bool {
    matches!(
        error,
        Error::Config(_)
            | Error::Validation(_)
            | Error::Deserialization(_)
            | Error::NotFound(_)
            | Error::FieldNotFound(_)
            | Error::InvalidSchema(_)
            | Error::InvalidFieldType(_)
    )
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
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
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

const fn http_method_name(method: HttpMethod) -> &'static str {
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

fn validate_workflow_push_outbox_size(data_bytes: &[u8]) -> r_data_core_core::error::Result<()> {
    if data_bytes.len() > WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "Workflow push payload body exceeds the maximum size of {WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES} bytes",
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_uses_base_for_first_attempt() {
        let policy = OutboxRetryPolicy::new(3, 2, 300);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 3);
    }

    #[test]
    fn retry_delay_scales_exponentially_until_cap() {
        let policy = OutboxRetryPolicy::new(2, 3, 20);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 2);
        assert_eq!(workflow_outbox_retry_delay_secs(2, &policy), 6);
        assert_eq!(workflow_outbox_retry_delay_secs(3, &policy), 18);
        assert_eq!(workflow_outbox_retry_delay_secs(4, &policy), 20);
        assert_eq!(workflow_outbox_retry_delay_secs(10, &policy), 20);
    }

    #[test]
    fn retry_delay_clamps_invalid_policy_values() {
        let policy = OutboxRetryPolicy::new(0, 1, 0);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 1);
        assert_eq!(workflow_outbox_retry_delay_secs(5, &policy), 1);
    }

    #[test]
    fn push_payload_size_validation_accepts_small_payload() {
        assert!(validate_workflow_push_outbox_size(&[0u8; 1024]).is_ok());
    }

    #[test]
    fn push_payload_size_validation_rejects_large_payload() {
        let payload = vec![0u8; WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES + 1];
        let result = validate_workflow_push_outbox_size(&payload);
        assert!(
            matches!(result, Err(r_data_core_core::error::Error::Validation(message)) if message.contains("maximum size"))
        );
    }
}
