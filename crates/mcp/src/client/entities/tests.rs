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
    RdcClient::new(&config, Arc::new(ApiKeyBackend::new("secret".to_string()))).expect("client")
}

fn ok(data: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "status": "success", "message": "ok", "meta": null, "data": data.clone()
    }))
}

async fn query_body(limit: Option<u32>, filter: Option<serde_json::Value>) -> serde_json::Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/customer/query"))
        .respond_with(ok(&json!([])))
        .mount(&server)
        .await;

    client_for(&server)
        .query_entities(
            "customer",
            filter.as_ref(),
            limit,
            &CallerContext::default(),
        )
        .await
        .expect("query should succeed");

    let requests = server.received_requests().await.expect("requests");
    serde_json::from_slice(&requests[0].body).expect("json body")
}

#[tokio::test]
async fn an_oversized_limit_is_clamped_not_rejected() {
    // A model asking for 5000 rows wants to see data; an error helps nobody.
    let body = query_body(Some(5000), None).await;
    assert_eq!(body["limit"], MAX_ENTITY_SAMPLE);
}

#[tokio::test]
async fn an_absent_limit_uses_the_sampling_default() {
    let body = query_body(None, None).await;
    assert_eq!(body["limit"], DEFAULT_ENTITY_SAMPLE);
}

#[tokio::test]
async fn a_modest_limit_is_passed_through() {
    let body = query_body(Some(3), None).await;
    assert_eq!(body["limit"], 3);
}

#[tokio::test]
async fn a_filter_is_forwarded_when_given() {
    let body = query_body(Some(5), Some(json!({ "status": "active" }))).await;
    assert_eq!(body["filters"]["status"], "active");
}

#[tokio::test]
async fn no_filter_key_is_sent_when_none_is_given() {
    // Sending "filters": null would be interpreted differently from omitting
    // it by some query handlers.
    let body = query_body(Some(5), None).await;
    assert!(body.get("filters").is_none(), "got {body}");
}

#[tokio::test]
async fn entity_fields_are_fetched_per_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/api/v1/entity-definitions/customer/fields"))
        .respond_with(ok(&json!([{ "name": "email", "field_type": "String" }])))
        .mount(&server)
        .await;

    let fields = client_for(&server)
        .get_entity_fields("customer", &CallerContext::default())
        .await
        .expect("fields should be readable");
    assert_eq!(fields[0]["name"], "email");
}
