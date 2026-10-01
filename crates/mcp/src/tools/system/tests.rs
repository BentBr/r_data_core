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
use crate::resources::{DSL_REFERENCE_URI, DSL_RULES_URI};

fn tools_with(server: &MockServer, permissions: Permissions) -> RdcTools {
    let mut map = HashMap::new();
    map.insert("RDC_BASE_URL".to_string(), server.uri());
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client");
    RdcTools::new(Arc::new(client), permissions, CallerContext::default())
}

fn perms(entries: &[&str]) -> Permissions {
    Permissions {
        is_super_admin: false,
        entries: entries.iter().map(|e| (*e).to_string()).collect(),
    }
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

async fn capabilities_mock(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/system/capabilities"))
        .respond_with(ok(&json!({
            "system_mail_configured": true,
            "workflow_mail_configured": true,
            "oidc_enabled": true
        })))
        .mount(server)
        .await;
}

// ── system_info ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn it_names_the_instance_it_is_pointed_at() {
    // The whole point of the tool: a model that cannot tell which deployment
    // it is connected to will happily write to the wrong one.
    let server = MockServer::start().await;
    capabilities_mock(&server).await;

    let tools = tools_with(&server, perms(&["workflows:read"]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains(&server.uri()), "{body}");
    assert!(body.contains(env!("CARGO_PKG_VERSION")), "{body}");
    assert!(body.contains("r-data-core-mcp"), "{body}");
}

#[tokio::test]
async fn it_reports_the_feature_flags_the_public_endpoint_exposes() {
    let server = MockServer::start().await;
    capabilities_mock(&server).await;

    let tools = tools_with(&server, perms(&[]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains("oidc_enabled"), "{body}");
}

#[tokio::test]
async fn it_omits_component_versions_from_a_caller_without_system_read() {
    // No mock for /system/versions at all: if the tool asked for it despite
    // the missing permission, wiremock would answer 404 and the assertion
    // below would still pass — so the request count is what is checked.
    let server = MockServer::start().await;
    capabilities_mock(&server).await;

    let tools = tools_with(&server, perms(&["workflows:read"]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains("\"versions\": null"), "{body}");
    let asked = server
        .received_requests()
        .await
        .expect("requests")
        .iter()
        .filter(|r| r.url.path().ends_with("/system/versions"))
        .count();
    assert_eq!(asked, 0, "a refused call was made anyway");
}

#[tokio::test]
async fn it_includes_component_versions_for_a_caller_who_may_read_them() {
    let server = MockServer::start().await;
    capabilities_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/system/versions"))
        .respond_with(ok(
            &json!({ "core": "0.4.11", "worker": null, "maintenance": null }),
        ))
        .mount(&server)
        .await;

    let tools = tools_with(&server, perms(&["system:read"]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains("0.4.11"), "{body}");
}

#[tokio::test]
async fn it_lists_only_the_tools_this_caller_may_use() {
    let server = MockServer::start().await;
    capabilities_mock(&server).await;

    let tools = tools_with(&server, perms(&["workflows:read"]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains("list_workflows"), "{body}");
    assert!(
        !body.contains("create_workflow"),
        "a tool the caller cannot use was advertised: {body}"
    );
}

#[tokio::test]
async fn an_unreachable_instance_is_reported_rather_than_faked() {
    // No mocks mounted: every call 404s. The tool must still answer, because
    // its output is what someone reads when diagnosing exactly this.
    let server = MockServer::start().await;
    let tools = tools_with(&server, perms(&[]));
    let body = text_of(&tools.system_info().await.expect("call"));

    assert!(body.contains("\"capabilities\": null"), "{body}");
}

// ── resources ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_rules_document_is_served_without_touching_the_network() {
    let server = MockServer::start().await;
    let tools = tools_with(&server, perms(&[]));

    let text = tools.resource_text(DSL_RULES_URI).await.expect("read");

    assert!(text.contains("previous_step"), "{text}");
    assert!(
        server
            .received_requests()
            .await
            .expect("requests")
            .is_empty(),
        "the static document should need no request"
    );
}

#[tokio::test]
async fn the_reference_is_rendered_from_the_live_catalogue() {
    let server = MockServer::start().await;
    for kind in ["from", "transform", "to"] {
        Mock::given(method("GET"))
            .and(path(format!("/admin/api/v1/dsl/{kind}/options")))
            .respond_with(ok(&json!({
                "types": [{
                    "type": format!("{kind}_only_type"),
                    "fields": [{
                        "name": "field", "type": "string",
                        "required": true, "options": null
                    }]
                }],
                "examples": []
            })))
            .mount(&server)
            .await;
    }

    let tools = tools_with(&server, perms(&[]));
    let text = tools.resource_text(DSL_REFERENCE_URI).await.expect("read");

    // All three groups, not just the one a lazy implementation would fetch.
    for kind in ["from", "transform", "to"] {
        assert!(text.contains(&format!("{kind}_only_type")), "{text}");
    }
}

#[tokio::test]
async fn an_unknown_uri_is_a_not_found_rather_than_an_empty_document() {
    let server = MockServer::start().await;
    let tools = tools_with(&server, perms(&[]));

    let error = tools
        .resource_text("rdatacore://dsl/nope")
        .await
        .expect_err("should not resolve");

    assert!(error.message.contains("rdatacore://dsl/nope"), "{error:?}");
}

#[tokio::test]
async fn an_unreadable_catalogue_fails_the_read_rather_than_serving_half_a_reference() {
    // A reference missing two thirds of the language is worse than an error:
    // a model would trust it and write against the third that rendered.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/dsl/from/options"))
        .respond_with(ok(&json!({ "types": [], "examples": [] })))
        .mount(&server)
        .await;

    let tools = tools_with(&server, perms(&[]));
    let result = tools.resource_text(DSL_REFERENCE_URI).await;

    assert!(result.is_err(), "a partial catalogue must not render");
}
