#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::auth::{ApiKeyBackend, CallerContext, Permissions};
use crate::client::RdcClient;
use crate::config::Config;

fn tools_for(server: &MockServer) -> RdcTools {
    let mut map = HashMap::new();
    map.insert("RDC_BASE_URL".to_string(), server.uri());
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client");
    RdcTools::new(
        Arc::new(client),
        Permissions {
            is_super_admin: true,
            entries: Vec::new(),
        },
        CallerContext::default(),
    )
}

fn ok(data: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "status": "success", "message": "ok", "meta": null, "data": data.clone()
    }))
}

fn text_of(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn mock_validate_ok(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/validate"))
        .respond_with(ok(&json!({ "valid": true })))
        .mount(server)
        .await;
}

async fn mock_validate_fails(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/validate"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "Invalid DSL",
            "violations": [{
                "field": "steps[0]",
                "json_path": "steps[0].to.type",
                "message": "unknown variant `entity_write`",
                "legal_values": ["format", "entity", "next_step", "email"]
            }]
        })))
        .mount(server)
        .await;
}

const WF: &str = "0195f0a0-0000-7000-8000-000000000001";

fn create_params() -> CreateWorkflowParams {
    CreateWorkflowParams {
        name: "test".to_string(),
        kind: "consumer".to_string(),
        steps: json!([]),
        description: None,
        enabled: None,
        schedule_cron: None,
    }
}

#[tokio::test]
async fn create_does_not_post_when_validation_fails() {
    let server = MockServer::start().await;
    mock_validate_fails(&server).await;
    // POST /workflows is deliberately NOT mocked: if the gate leaks, the
    // request 404s and this test fails loudly rather than silently passing.

    let result = tools_for(&server)
        .create_workflow(Parameters(create_params()))
        .await
        .expect("tool call");

    assert_eq!(result.is_error, Some(true));
    let text = text_of(&result);
    assert!(text.contains("steps[0].to.type"), "must locate it: {text}");
    assert!(text.contains("next_step"), "must list alternatives: {text}");

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(
        requests.len(),
        1,
        "only the validation call should have been made; the create must not \
         have been attempted: {requests:?}"
    );
}

#[tokio::test]
async fn create_defaults_to_disabled() {
    let server = MockServer::start().await;
    mock_validate_ok(&server).await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!({ "uuid": WF })))
        .mount(&server)
        .await;

    tools_for(&server)
        .create_workflow(Parameters(create_params()))
        .await
        .expect("create should succeed");

    let requests = server.received_requests().await.expect("requests");
    let create = requests
        .iter()
        .find(|r| r.url.path() == "/admin/api/v1/workflows")
        .expect("create request");
    let body: serde_json::Value = serde_json::from_slice(&create.body).expect("json");
    assert_eq!(
        body["enabled"], false,
        "a workflow that goes live the moment it is created has skipped every \
         check that would have caught a mistake: {body}"
    );
}

#[tokio::test]
async fn create_honours_an_explicit_enabled_true() {
    let server = MockServer::start().await;
    mock_validate_ok(&server).await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!({ "uuid": WF })))
        .mount(&server)
        .await;

    let mut params = create_params();
    params.enabled = Some(true);
    tools_for(&server)
        .create_workflow(Parameters(params))
        .await
        .expect("create should succeed");

    let requests = server.received_requests().await.expect("requests");
    let create = requests
        .iter()
        .find(|r| r.url.path() == "/admin/api/v1/workflows")
        .expect("create request");
    let body: serde_json::Value = serde_json::from_slice(&create.body).expect("json");
    assert_eq!(body["enabled"], true);
}

#[tokio::test]
async fn a_bare_steps_array_is_wrapped_into_a_config_object() {
    let server = MockServer::start().await;
    mock_validate_ok(&server).await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!({ "uuid": WF })))
        .mount(&server)
        .await;

    tools_for(&server)
        .create_workflow(Parameters(create_params()))
        .await
        .expect("create should succeed");

    let requests = server.received_requests().await.expect("requests");
    let create = requests
        .iter()
        .find(|r| r.url.path() == "/admin/api/v1/workflows")
        .expect("create request");
    let body: serde_json::Value = serde_json::from_slice(&create.body).expect("json");
    assert!(
        body["config"]["steps"].is_array(),
        "the stored config needs a steps object: {body}"
    );
}

#[tokio::test]
async fn an_already_wrapped_config_is_not_double_wrapped() {
    // A model that passes { "steps": [...] } should not get
    // { "steps": { "steps": [...] } } and a baffling error.
    let server = MockServer::start().await;
    mock_validate_ok(&server).await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!({ "uuid": WF })))
        .mount(&server)
        .await;

    let mut params = create_params();
    params.steps = json!({ "steps": [] });
    tools_for(&server)
        .create_workflow(Parameters(params))
        .await
        .expect("create should succeed");

    let requests = server.received_requests().await.expect("requests");
    let create = requests
        .iter()
        .find(|r| r.url.path() == "/admin/api/v1/workflows")
        .expect("create request");
    let body: serde_json::Value = serde_json::from_slice(&create.body).expect("json");
    assert!(body["config"]["steps"].is_array(), "double-wrapped: {body}");
}

#[tokio::test]
async fn update_does_not_put_when_validation_fails() {
    let server = MockServer::start().await;
    mock_validate_fails(&server).await;

    let result = tools_for(&server)
        .update_workflow(Parameters(UpdateWorkflowParams {
            uuid: WF.to_string(),
            name: "test".to_string(),
            kind: "consumer".to_string(),
            steps: json!([]),
            description: None,
            enabled: None,
            schedule_cron: None,
        }))
        .await
        .expect("tool call");

    assert_eq!(result.is_error, Some(true));
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1, "the update must not have been attempted");
}

#[tokio::test]
async fn restore_is_one_call_and_says_it_appended() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/admin/api/v1/workflows/{WF}/versions/3/restore"
        )))
        .respond_with(ok(&json!({})))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .restore_workflow_version(Parameters(VersionParams {
            uuid: WF.to_string(),
            version_number: 3,
        }))
        .await
        .expect("restore should succeed");

    assert!(
        text_of(&result).contains("not rewound"),
        "the model must report the semantics accurately: {}",
        text_of(&result)
    );
    assert_eq!(
        server.received_requests().await.expect("requests").len(),
        1,
        "restore belongs to the server, not a client-side fetch-then-update"
    );
}

#[tokio::test]
async fn a_successful_validation_still_points_at_the_dry_run() {
    // A model that stops at a green validate has not caught the mapping and
    // type errors validation cannot see.
    let server = MockServer::start().await;
    mock_validate_ok(&server).await;

    let result = tools_for(&server)
        .validate_dsl(Parameters(ValidateDslParams { steps: json!([]) }))
        .await
        .expect("tool call");

    assert!(
        text_of(&result).contains("test_workflow"),
        "{}",
        text_of(&result)
    );
}

#[tokio::test]
async fn a_malformed_uuid_never_reaches_the_server() {
    let server = MockServer::start().await;
    let result = tools_for(&server)
        .list_workflow_versions(Parameters(WorkflowUuidParams {
            uuid: "nope".to_string(),
        }))
        .await
        .expect("tool call");

    assert_eq!(result.is_error, Some(true));
    assert!(server
        .received_requests()
        .await
        .expect("requests")
        .is_empty());
}
