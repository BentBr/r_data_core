#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
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

#[test]
fn an_unknown_options_kind_is_rejected_with_the_legal_values() {
    let err = OptionsKind::parse("source").expect_err("not a kind");
    match err {
        ClientError::Validation { legal_values, .. } => {
            assert_eq!(legal_values, vec!["from", "to", "transform"]);
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn the_three_kinds_parse() {
    assert_eq!(OptionsKind::parse("from").expect("from"), OptionsKind::From);
    assert_eq!(OptionsKind::parse("to").expect("to"), OptionsKind::To);
    assert_eq!(
        OptionsKind::parse("transform").expect("transform"),
        OptionsKind::Transform
    );
}

#[tokio::test]
async fn options_are_fetched_from_the_matching_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/dsl/to/options"))
        .respond_with(ok(&json!({
            "types": [{ "type": "entity", "fields": [] }],
            "examples": []
        })))
        .mount(&server)
        .await;

    let options = client_for(&server)
        .dsl_options(OptionsKind::To, &CallerContext::default())
        .await
        .expect("options should be readable");
    assert_eq!(options.types[0].r#type, "entity");
}

#[tokio::test]
async fn dry_run_sends_both_the_program_and_the_input() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/dry-run"))
        .respond_with(ok(&json!({ "steps": [], "would_write": [] })))
        .mount(&server)
        .await;

    client_for(&server)
        .dry_run(
            &json!([]),
            &json!({ "price": 100 }),
            &CallerContext::default(),
        )
        .await
        .expect("dry run should succeed");

    let requests = server.received_requests().await.expect("requests");
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json");
    assert!(body.get("steps").is_some(), "got {body}");
    assert_eq!(body["input"]["price"], 100);
}

#[tokio::test]
async fn an_invalid_program_surfaces_as_a_located_validation_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/validate"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "Invalid DSL",
            "violations": [{
                "field": "steps[0]",
                "json_path": "steps[0].to.type",
                "message": "unknown variant `nope`",
                "legal_values": ["format", "entity"]
            }]
        })))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .validate_dsl(&json!([]), &CallerContext::default())
        .await
        .expect_err("should be invalid");
    assert!(err.to_tool_message().contains("steps[0].to.type"));
}
