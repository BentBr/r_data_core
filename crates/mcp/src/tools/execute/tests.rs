#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::auth::{ApiKeyBackend, Permissions};
use crate::client::RdcClient;
use crate::config::Config;

fn tools_for(server: &MockServer) -> RdcTools {
    let mut map = HashMap::new();
    map.insert("RDC_BASE_URL".to_string(), server.uri());
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(&config, Arc::new(ApiKeyBackend::new("secret".to_string())))
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

const WF: &str = "0195f0a0-0000-7000-8000-000000000001";
const RUN: &str = "0195f0a0-0000-7000-8000-0000000000aa";

fn run_summary(status: &str) -> serde_json::Value {
    json!({
        "uuid": RUN, "status": status, "queued_at": null, "started_at": null,
        "finished_at": null, "processed_items": 3, "failed_items": 0
    })
}

#[tokio::test]
async fn test_workflow_returns_the_trace() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/dry-run"))
        .respond_with(ok(&json!({
            "steps": [{
                "step_index": 0,
                "produced": { "total": 119.0 },
                "to": { "type": "format" },
                "effect": "no_side_effect",
                "suppressed_detail": null
            }],
            "would_write": []
        })))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .test_workflow(Parameters(TestWorkflowParams {
            steps: Some(json!([])),
            uuid: None,
            input: json!({ "price": 100 }),
        }))
        .await
        .expect("tool call");

    assert_ne!(result.is_error, Some(true), "{}", text_of(&result));
    assert!(text_of(&result).contains("119"), "{}", text_of(&result));
}

#[tokio::test]
async fn test_workflow_can_take_a_saved_workflows_program() {
    // Debugging an existing workflow should not require fetching the program
    // and pasting it back.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/{WF}")))
        .respond_with(ok(&json!({
            "uuid": WF, "name": "wf", "description": null, "kind": "consumer",
            "enabled": true, "schedule_cron": null,
            "config": { "steps": [{ "marker": true }] },
            "versioning_disabled": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/dry-run"))
        .respond_with(ok(&json!({ "steps": [], "would_write": [] })))
        .mount(&server)
        .await;

    tools_for(&server)
        .test_workflow(Parameters(TestWorkflowParams {
            steps: None,
            uuid: Some(WF.to_string()),
            input: json!({}),
        }))
        .await
        .expect("tool call");

    let requests = server.received_requests().await.expect("requests");
    let dry_run = requests
        .iter()
        .find(|r| r.url.path() == "/admin/api/v1/dsl/dry-run")
        .expect("dry-run request");
    let body: serde_json::Value = serde_json::from_slice(&dry_run.body).expect("json");
    assert_eq!(
        body["steps"][0]["marker"], true,
        "the saved program must be unwrapped from its config: {body}"
    );
}

#[tokio::test]
async fn test_workflow_without_steps_or_uuid_explains_what_is_needed() {
    let server = MockServer::start().await;
    let result = tools_for(&server)
        .test_workflow(Parameters(TestWorkflowParams {
            steps: None,
            uuid: None,
            input: json!({}),
        }))
        .await
        .expect("tool call");

    assert_eq!(result.is_error, Some(true));
    let text = text_of(&result);
    assert!(text.contains("steps"), "{text}");
    assert!(text.contains("uuid"), "{text}");
}

#[tokio::test]
async fn run_workflow_waits_and_returns_status_with_logs() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/admin/api/v1/workflows/{WF}/run")))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({
            "status": "success", "message": "queued", "meta": null,
            "data": { "uuid": RUN }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/{WF}/runs")))
        .respond_with(ok(&json!([run_summary("success")])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/runs/{RUN}/logs")))
        .respond_with(ok(&json!([{
            "uuid": RUN, "ts": "2026-09-06T10:00:00Z", "level": "info",
            "message": "processed 3 items", "meta": null
        }])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .run_workflow(Parameters(RunWorkflowParams {
            uuid: WF.to_string(),
            input: None,
            wait: None,
            timeout_secs: Some(5),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert!(text.contains("success"), "{text}");
    assert!(
        text.contains("processed 3 items"),
        "status and logs must arrive together so the model does not need a \
         second turn: {text}"
    );
}

#[tokio::test]
async fn run_workflow_can_return_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/admin/api/v1/workflows/{WF}/run")))
        .respond_with(ok(&json!({ "uuid": RUN })))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .run_workflow(Parameters(RunWorkflowParams {
            uuid: WF.to_string(),
            input: None,
            wait: Some(false),
            timeout_secs: None,
        }))
        .await
        .expect("tool call");
    assert!(text_of(&result).contains("started"));
}

#[tokio::test]
async fn a_run_that_outlives_the_wait_warns_against_running_again() {
    // Re-running a workflow that is merely slow doubles its side effects.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/admin/api/v1/workflows/{WF}/run")))
        .respond_with(ok(&json!({ "uuid": RUN })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/{WF}/runs")))
        .respond_with(ok(&json!([run_summary("running")])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .run_workflow(Parameters(RunWorkflowParams {
            uuid: WF.to_string(),
            input: None,
            wait: Some(true),
            timeout_secs: Some(0),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert!(text.contains("timed_out"), "{text}");
    assert!(text.contains("do NOT run the workflow again"), "{text}");
}

#[tokio::test]
async fn list_runs_honours_the_status_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows/runs"))
        .respond_with(ok(&json!([run_summary("success"), run_summary("failed")])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .list_runs(Parameters(ListRunsParams {
            uuid: None,
            status: Some("failed".to_string()),
            limit: None,
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert!(text.contains("failed"), "{text}");
    assert!(
        !text.contains("success"),
        "the filter must not be silently ignored: {text}"
    );
}

#[tokio::test]
async fn get_run_logs_honours_the_level_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/runs/{RUN}/logs")))
        .respond_with(ok(&json!([
            { "uuid": RUN, "ts": "t", "level": "info", "message": "fine", "meta": null },
            { "uuid": RUN, "ts": "t", "level": "error", "message": "broke", "meta": null }
        ])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .get_run_logs(Parameters(RunLogsParams {
            run_uuid: RUN.to_string(),
            level: Some("error".to_string()),
            limit: None,
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert!(text.contains("broke"), "{text}");
    assert!(!text.contains("fine"), "{text}");
}

#[test]
fn the_terminal_statuses_match_the_engine() {
    // RunStatus serializes lowercase; queued and running are not terminal.
    assert!(TERMINAL_STATUSES.contains(&"success"));
    assert!(TERMINAL_STATUSES.contains(&"failed"));
    assert!(TERMINAL_STATUSES.contains(&"cancelled"));
    assert!(!TERMINAL_STATUSES.contains(&"running"));
    assert!(!TERMINAL_STATUSES.contains(&"queued"));
}
