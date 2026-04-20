#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use base64::Engine as _;
use sha2::Digest;
use uuid::Uuid;

use r_data_core_core::error::Error;
use r_data_core_workflow::data::adapters::destination::HttpMethod;

use super::payload::destination_method_name;

pub(super) const fn is_permanent_outbox_failure(error: &Error) -> bool {
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

pub(super) fn parse_http_method(value: &str) -> Option<HttpMethod> {
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

pub(super) fn workflow_push_fingerprint(
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    item_uuid: Uuid,
    destination_type: &str,
    destination_config: &serde_json::Value,
    method: Option<&HttpMethod>,
    data_bytes: &[u8],
) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(workflow_uuid.as_bytes());
    hasher.update(run_uuid.as_bytes());
    hasher.update(item_uuid.as_bytes());
    hasher.update(destination_type.as_bytes());
    hasher.update(destination_config.to_string().as_bytes());
    if let Some(method) = method {
        hasher.update(destination_method_name(*method).as_bytes());
    }
    hasher.update(data_bytes);
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}
