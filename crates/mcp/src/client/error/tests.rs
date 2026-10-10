#![allow(clippy::expect_used)]

//! Tests for the error messages a model reads.
//!
//! These assert on message *content*, which normally makes for brittle tests.
//! Here the content is the feature: a message that omits the path or the legal
//! values has failed at its job even though the error type is correct.

use super::*;
use serde_json::json;

/// The shape `unprocessable_entity_with_violations` actually produces —
/// violations at the top level, beside `message`, not under `meta`. Verified
/// against the running endpoint.
fn violation_body(json_path: &str, message: &str, legal: &[&str]) -> serde_json::Value {
    json!({
        "message": "Invalid DSL",
        "violations": [{
            "field": "steps[0]",
            "json_path": json_path,
            "code": "DSL_STEP_MALFORMED",
            "message": message,
            "legal_values": legal,
        }]
    })
}

#[test]
fn parses_a_real_validation_envelope() {
    let body = violation_body(
        "steps[0].to.type",
        "unknown variant `entity_write`",
        &["format", "entity", "next_step", "email"],
    );

    match ClientError::from_api_body(422, &body, "https://rdc.example.com") {
        ClientError::Validation {
            json_path,
            legal_values,
            message,
        } => {
            assert_eq!(json_path.as_deref(), Some("steps[0].to.type"));
            assert!(legal_values.contains(&"next_step".to_string()));
            assert!(message.contains("entity_write"));
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn a_validation_message_names_the_path_and_the_alternatives() {
    let body = violation_body(
        "steps[0].to.type",
        "unknown variant `entity_write`",
        &["format", "entity"],
    );
    let msg = ClientError::from_api_body(422, &body, "https://rdc.example.com").to_tool_message();

    assert!(msg.contains("steps[0].to.type"), "must locate it: {msg}");
    assert!(
        msg.contains("entity_write"),
        "must quote the bad value: {msg}"
    );
    assert!(msg.contains("format"), "must list alternatives: {msg}");
}

#[test]
fn falls_back_to_the_field_when_there_is_no_json_path() {
    // Producers other than the DSL validator supply only `field`.
    let body = json!({
        "message": "Validation failed",
        "violations": [{ "field": "schedule_cron", "message": "Invalid cron expression" }]
    });
    match ClientError::from_api_body(422, &body, "https://rdc.example.com") {
        ClientError::Validation { json_path, .. } => {
            assert_eq!(json_path.as_deref(), Some("schedule_cron"));
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn recovers_alternatives_from_prose_when_they_are_not_supplied_as_data() {
    // A server predating structured legal_values still says them in the
    // message; losing them there would silently degrade the tool errors.
    let body = json!({
        "message": "Invalid DSL",
        "violations": [{
            "field": "steps[0]",
            "message": "unknown variant `nope`, expected one of `format`, `entity`"
        }]
    });
    match ClientError::from_api_body(422, &body, "https://rdc.example.com") {
        ClientError::Validation { legal_values, .. } => {
            assert_eq!(
                legal_values,
                vec!["format".to_string(), "entity".to_string()]
            );
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn forbidden_names_the_permission_and_does_not_invite_a_retry() {
    let err = ClientError::from_api_body(
        403,
        &json!({ "message": "Insufficient permissions: workflows:write required" }),
        "https://rdc.example.com",
    );
    let msg = err.to_tool_message();

    assert!(msg.contains("workflows:write"), "must name it: {msg}");
    let lowered = msg.to_lowercase();
    assert!(
        !lowered.contains("try again") && !lowered.contains("retry"),
        "a 403 is not transient; suggesting a retry produces a loop: {msg}"
    );
}

#[test]
fn a_path_scoped_permission_survives_extraction() {
    let err = ClientError::from_api_body(
        403,
        &json!({ "message": "Insufficient permissions: entities:/customers:read required" }),
        "https://rdc.example.com",
    );
    assert!(err.to_tool_message().contains("entities:/customers:read"));
}

#[test]
fn a_forbidden_without_a_named_permission_still_reads_sensibly() {
    let err = ClientError::from_api_body(
        403,
        &json!({ "message": "Forbidden" }),
        "https://rdc.example.com",
    );
    assert!(err.to_tool_message().contains("Permission denied"));
}

#[test]
fn a_server_error_tells_the_caller_not_to_retry() {
    let err = ClientError::from_api_body(
        500,
        &json!({ "message": "database unavailable" }),
        "https://rdc.example.com",
    );
    let msg = err.to_tool_message();
    assert!(msg.contains("database unavailable"));
    assert!(
        msg.contains("do not retry"),
        "an automatic retry on a write can duplicate it: {msg}"
    );
}

#[test]
fn a_timeout_warns_against_starting_a_second_run() {
    let msg = ClientError::Timeout.to_tool_message();
    assert!(
        msg.contains("still be executing"),
        "re-running a workflow that is merely slow doubles its side effects: {msg}"
    );
}

#[test]
fn unreachable_names_the_url() {
    let err = ClientError::Unreachable {
        url: "https://rdc.example.com".to_string(),
    };
    assert!(err.to_tool_message().contains("https://rdc.example.com"));
}

#[test]
fn tolerates_a_body_that_is_not_an_envelope() {
    // Actix's own error handlers, and any proxy in front of the server, can
    // return something else entirely. Never panic on it.
    for body in [
        json!("upstream timeout"),
        json!(null),
        json!([1, 2, 3]),
        json!({ "unexpected": true }),
    ] {
        let err = ClientError::from_api_body(504, &body, "https://rdc.example.com");
        assert!(!err.to_tool_message().is_empty(), "got nothing for {body}");
    }
}

#[test]
fn an_unmapped_status_is_reported_with_its_code() {
    let err = ClientError::from_api_body(418, &json!({}), "https://rdc.example.com");
    assert!(err.to_tool_message().contains("418"));
}

#[test]
fn legal_values_are_empty_for_an_unrelated_message() {
    assert_eq!(
        legal_values_from("invalid type: string, expected u32"),
        Vec::<String>::new()
    );
}
