#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::auth::ApiKeyBackend;
use crate::config::Config;

fn client_for(server: &MockServer) -> RdcClient {
    let mut map = HashMap::new();
    map.insert("RDC_BASE_URL".to_string(), server.uri());
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client")
}

fn ok(data: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "status": "success", "message": "ok", "meta": null, "data": data.clone()
    }))
}

const WF: &str = "0195f0a0-0000-7000-8000-000000000001";

#[tokio::test]
async fn list_workflows_deserializes_the_summaries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .and(query_param("per_page", "10"))
        .respond_with(ok(&json!([{
            "uuid": WF,
            "name": "nightly-import",
            "kind": "consumer",
            "enabled": true,
            "schedule_cron": "0 2 * * *",
            "has_api_endpoint": false,
            "versioning_disabled": false
        }])))
        .mount(&server)
        .await;

    let workflows = client_for(&server)
        .list_workflows(Some(10), None, &CallerContext::default())
        .await
        .expect("list should succeed");

    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].name, "nightly-import");
    assert!(workflows[0].enabled);
}

#[tokio::test]
async fn get_workflow_returns_the_program() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/{WF}")))
        .respond_with(ok(&json!({
            "uuid": WF,
            "name": "nightly-import",
            "description": null,
            "kind": "consumer",
            "enabled": true,
            "schedule_cron": null,
            "config": { "steps": [] },
            "versioning_disabled": false
        })))
        .mount(&server)
        .await;

    let detail = client_for(&server)
        .get_workflow(WF.parse().expect("uuid"), &CallerContext::default())
        .await
        .expect("get should succeed");
    assert!(detail.config["steps"].is_array());
}

#[tokio::test]
async fn restore_is_a_single_call_to_the_restore_endpoint() {
    // Restore semantics belong to the server. If this ever becomes a
    // fetch-then-update pair, the logic has leaked into the wrong layer.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/admin/api/v1/workflows/{WF}/versions/3/restore"
        )))
        .respond_with(ok(&json!({})))
        .mount(&server)
        .await;

    client_for(&server)
        .restore_workflow_version(WF.parse().expect("uuid"), 3, &CallerContext::default())
        .await
        .expect("restore should succeed");

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(
        requests.len(),
        1,
        "restore must be one call, not a client-side fetch-then-update: {requests:?}"
    );
}

#[tokio::test]
async fn run_logs_use_the_route_that_actually_exists() {
    // The handler's OpenAPI annotation used to say /workflow-runs/... which is
    // not a route the router serves. Pinning the real one.
    let run = "0195f0a0-0000-7000-8000-0000000000aa";
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/admin/api/v1/workflows/runs/{run}/logs")))
        .respond_with(ok(&json!([{
            "uuid": run,
            "ts": "2026-09-06T10:00:00Z",
            "level": "info",
            "message": "processed 3 items",
            "meta": null
        }])))
        .mount(&server)
        .await;

    let logs = client_for(&server)
        .get_run_logs(run.parse().expect("uuid"), None, &CallerContext::default())
        .await
        .expect("logs should be readable");
    assert_eq!(logs[0].level, "info");
}

#[tokio::test]
async fn list_runs_without_a_uuid_uses_the_cross_workflow_route() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows/runs"))
        .respond_with(ok(&json!([])))
        .mount(&server)
        .await;

    let runs = client_for(&server)
        .list_runs(None, None, &CallerContext::default())
        .await
        .expect("list should succeed");
    assert!(runs.is_empty());
}

#[tokio::test]
async fn a_cron_expression_is_percent_encoded() {
    // "0 2 * * *" raw in a query string is not parseable by the server.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows/cron/preview"))
        .and(query_param("expression", "0 2 * * *"))
        .respond_with(ok(&json!({ "next": [] })))
        .mount(&server)
        .await;

    client_for(&server)
        .preview_cron("0 2 * * *", &CallerContext::default())
        .await
        .expect("the server must receive a decodable expression");
}

#[test]
fn urlencode_escapes_what_a_query_string_cannot_carry() {
    assert_eq!(urlencode("0 2 * * *"), "0+2+%2A+%2A+%2A");
    assert_eq!(urlencode("plain-value_1.0~"), "plain-value_1.0~");
}
