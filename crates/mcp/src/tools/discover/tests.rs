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

/// The text a tool result carries, whether success or error.
fn text_of(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

const WF: &str = "0195f0a0-0000-7000-8000-000000000001";

fn summary(name: &str, enabled: bool) -> serde_json::Value {
    json!({
        "uuid": WF, "name": name, "kind": "consumer", "enabled": enabled,
        "schedule_cron": null, "has_api_endpoint": false, "versioning_disabled": false
    })
}

#[tokio::test]
async fn list_workflows_returns_them() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!([summary("nightly", true)])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .list_workflows(Parameters(ListWorkflowsParams {
            limit: None,
            offset: None,
            enabled: None,
        }))
        .await
        .expect("tool call");

    assert_ne!(result.is_error, Some(true), "got {}", text_of(&result));
    assert!(text_of(&result).contains("nightly"));
}

#[tokio::test]
async fn list_workflows_honours_the_enabled_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ok(&json!([
            summary("live-one", true),
            summary("disabled-one", false)
        ])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .list_workflows(Parameters(ListWorkflowsParams {
            limit: None,
            offset: None,
            enabled: Some(false),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert!(text.contains("disabled-one"), "got {text}");
    assert!(
        !text.contains("live-one"),
        "the flag must not be silently ignored: {text}"
    );
}

#[tokio::test]
async fn a_failure_is_a_readable_tool_error_not_an_opaque_protocol_error() {
    // MCP renders Err(ErrorData) opaquely, which would throw away the text
    // written to help a model correct itself. Failures must arrive as
    // CallToolResult::error instead.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "message": "Insufficient permissions: workflows:read required"
        })))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .list_workflows(Parameters(ListWorkflowsParams {
            limit: None,
            offset: None,
            enabled: None,
        }))
        .await
        .expect("the call itself succeeds; the tool reports the failure");

    assert_eq!(result.is_error, Some(true));
    assert!(
        text_of(&result).contains("workflows:read"),
        "the permission must reach the model: {}",
        text_of(&result)
    );
}

#[tokio::test]
async fn a_malformed_uuid_is_explained_rather_than_sent_to_the_server() {
    let server = MockServer::start().await;
    let result = tools_for(&server)
        .get_workflow(Parameters(GetWorkflowParams {
            uuid: "not-a-uuid".to_string(),
        }))
        .await
        .expect("tool call");

    assert_eq!(result.is_error, Some(true));
    assert!(text_of(&result).contains("not a valid UUID"));
    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty(),
        "a client-side validation failure must not hit the server"
    );
}

#[tokio::test]
async fn an_unknown_dsl_kind_lists_the_legal_values() {
    let server = MockServer::start().await;
    let result = tools_for(&server)
        .dsl_options(Parameters(DslOptionsParams {
            kind: "source".to_string(),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert_eq!(result.is_error, Some(true));
    assert!(text.contains("from"), "got {text}");
    assert!(text.contains("transform"), "got {text}");
}

#[tokio::test]
async fn query_entities_caps_the_sample_size() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/customer/query"))
        .respond_with(ok(&json!([])))
        .mount(&server)
        .await;

    tools_for(&server)
        .query_entities(Parameters(QueryEntitiesParams {
            entity_type: "customer".to_string(),
            filter: None,
            limit: Some(5000),
        }))
        .await
        .expect("tool call");

    let requests = server.received_requests().await.expect("requests");
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json");
    assert_eq!(body["limit"], MAX_ENTITY_SAMPLE);
}

#[tokio::test]
async fn entity_fields_are_returned_for_the_named_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/entity-definitions/customer/fields"))
        .respond_with(ok(&json!([{ "name": "email" }])))
        .mount(&server)
        .await;

    let result = tools_for(&server)
        .get_entity_definition(Parameters(GetEntityDefinitionParams {
            entity_type: "customer".to_string(),
        }))
        .await
        .expect("tool call");
    assert!(text_of(&result).contains("email"));
}
