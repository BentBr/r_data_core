#![allow(clippy::expect_used)]

use std::collections::HashMap;

use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::auth::ApiKeyBackend;

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

#[tokio::test]
async fn sends_the_admin_token_and_decodes_the_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        // A bearer, not the API key: the admin API takes a JWT, and the key
        // is exchanged for one before any of this.
        .and(header("Authorization", "Bearer an-admin-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success", "message": "ok", "meta": null,
            "data": [{ "name": "nightly" }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let envelope: Envelope<Vec<serde_json::Value>> = client
        .get_json("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect("request should succeed");

    let data = envelope.require_data().expect("data present");
    assert_eq!(data[0]["name"], "nightly");
}

#[tokio::test]
async fn a_successful_response_with_no_data_is_a_clear_error_not_a_panic() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success", "message": "nothing here", "data": null, "meta": null
        })))
        .mount(&server)
        .await;

    let envelope: Envelope<Vec<serde_json::Value>> = client_for(&server)
        .get_json("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect("the request itself succeeded");

    let err = envelope.require_data().expect_err("no data to return");
    assert!(
        err.to_tool_message().contains("nothing here"),
        "the envelope's message is the only clue available: {err:?}"
    );
}

#[tokio::test]
async fn posts_the_body_it_is_given() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/validate"))
        .and(body_json(json!({ "steps": [] })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success", "message": "ok", "meta": null,
            "data": { "valid": true }
        })))
        .mount(&server)
        .await;

    let envelope: Envelope<serde_json::Value> = client_for(&server)
        .post_json(
            "/admin/api/v1/dsl/validate",
            &json!({ "steps": [] }),
            &CallerContext::default(),
        )
        .await
        .expect("request should succeed");
    assert_eq!(envelope.require_data().expect("data")["valid"], true);
}

#[tokio::test]
async fn maps_403_to_a_forbidden_error_naming_the_permission() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "status": "error", "message": "Insufficient permissions: workflows:read required",
            "data": null, "meta": null
        })))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .get_json::<serde_json::Value>("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect_err("should be forbidden");

    assert!(matches!(err, ClientError::Forbidden { .. }), "got {err:?}");
    assert!(err.to_tool_message().contains("workflows:read"));
}

#[tokio::test]
async fn maps_422_violations_to_a_located_validation_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/dsl/validate"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "Invalid DSL",
            "violations": [{
                "field": "steps[0]",
                "json_path": "steps[0].to.type",
                "message": "unknown variant `entity_write`",
                "legal_values": ["format", "entity"]
            }]
        })))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .post_json::<_, serde_json::Value>(
            "/admin/api/v1/dsl/validate",
            &json!({}),
            &CallerContext::default(),
        )
        .await
        .expect_err("should be a validation error");

    match err {
        ClientError::Validation {
            json_path,
            legal_values,
            ..
        } => {
            assert_eq!(json_path.as_deref(), Some("steps[0].to.type"));
            assert!(legal_values.contains(&"entity".to_string()));
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[tokio::test]
async fn a_non_json_error_body_still_produces_a_usable_error() {
    // A reverse proxy returning an HTML 502 must not turn into a decode panic.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>"))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .get_json::<serde_json::Value>("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect_err("should fail");
    assert!(err.to_tool_message().contains("bad gateway"), "got {err:?}");
}

#[tokio::test]
async fn an_undecodable_success_body_reports_what_it_saw() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .get_json::<serde_json::Value>("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect_err("should fail to decode");
    assert!(
        err.to_tool_message().contains("not json at all"),
        "a decode error without the body is undebuggable: {err:?}"
    );
}

#[tokio::test]
async fn an_unreachable_server_names_the_url() {
    let mut map = HashMap::new();
    // Port 1 on loopback: reliably refused, and fast.
    map.insert("RDC_BASE_URL".to_string(), "http://127.0.0.1:1".to_string());
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client");

    let err = client
        .get_json::<serde_json::Value>("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect_err("nothing is listening");
    assert!(
        matches!(err, ClientError::Unreachable { .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn a_base_url_with_a_path_prefix_is_preserved() {
    // Url::join would drop "/rdc" when given a leading-slash path, silently
    // pointing every request at the wrong place behind a reverse proxy.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rdc/admin/api/v1/workflows"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success", "message": "ok", "data": [], "meta": null
        })))
        .mount(&server)
        .await;

    let mut map = HashMap::new();
    map.insert("RDC_BASE_URL".to_string(), format!("{}/rdc", server.uri()));
    map.insert("RDC_API_KEY".to_string(), "secret".to_string());
    let config = Config::from_map(&map).expect("config");
    let client = RdcClient::new(
        &config,
        Arc::new(ApiKeyBackend::holding(&config, "an-admin-token").expect("backend")),
    )
    .expect("client");

    let envelope: Envelope<Vec<serde_json::Value>> = client
        .get_json("/admin/api/v1/workflows", &CallerContext::default())
        .await
        .expect("the prefix must be kept");
    assert_eq!(
        envelope.require_data().expect("data"),
        Vec::<serde_json::Value>::new()
    );
}
