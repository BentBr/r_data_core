#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use r_data_core_core::error::Error;
use r_data_core_workflow::data::adapters::destination::HttpMethod;

use super::WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPushOutboxPayload {
    pub workflow_id: Uuid,
    pub run_uuid: Uuid,
    pub item_uuid: Uuid,
    #[serde(default)]
    pub destination_step_index: Option<usize>,
    pub destination_type: String,
    pub destination_config: serde_json::Value,
    #[serde(default)]
    pub auth_required: bool,
    #[serde(default)]
    pub destination_auth: Option<serde_json::Value>,
    pub method: Option<String>,
    pub format_type: String,
    pub data_base64: String,
}

pub fn validate_workflow_push_outbox_size(
    data_bytes: &[u8],
) -> r_data_core_core::error::Result<()> {
    if data_bytes.len() > WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES {
        return Err(Error::Validation(format!(
            "Workflow push payload body exceeds the maximum size of {WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES} bytes",
        )));
    }

    Ok(())
}

pub(super) fn encode_workflow_push_payload(data_bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data_bytes)
}

pub(super) const fn destination_method_name(method: HttpMethod) -> &'static str {
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
