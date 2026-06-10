use uuid::Uuid;

use r_data_core_core::outbox::{WORKFLOW_PUSH_ENQUEUE_KIND, WORKFLOW_PUSH_TOPIC};
use r_data_core_persistence::OutboxRepositoryTrait;
use r_data_core_workflow::data::adapters::destination::HttpMethod;

use super::super::payload::{
    destination_method_name, encode_workflow_push_payload, WorkflowPushOutboxPayload,
};
use super::super::support::workflow_push_fingerprint;
use super::super::validate_workflow_push_outbox_size;

/// Enqueue a workflow push delivery in the outbox.
///
/// # Errors
/// Returns an error if the payload is too large or the insert fails.
#[allow(clippy::too_many_arguments)]
pub async fn enqueue_workflow_push_outbox(
    outbox_repo: &dyn OutboxRepositoryTrait,
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    item_uuid: Uuid,
    destination_step_index: usize,
    destination_type: &str,
    destination_config: serde_json::Value,
    auth_required: bool,
    method: Option<HttpMethod>,
    format_type: &str,
    data_bytes: &[u8],
) -> r_data_core_core::error::Result<Uuid> {
    validate_workflow_push_outbox_size(data_bytes)?;
    let payload = WorkflowPushOutboxPayload {
        workflow_id: workflow_uuid,
        run_uuid,
        item_uuid,
        destination_step_index: Some(destination_step_index),
        destination_type: destination_type.to_string(),
        destination_config: destination_config.clone(),
        auth_required,
        destination_auth: None,
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
