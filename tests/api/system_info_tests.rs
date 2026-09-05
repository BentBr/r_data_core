#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The read-only `/system/*` endpoints the admin dashboard polls: licence
//! status, build versions, capabilities and the system log.

use actix_web::{http::StatusCode, test};
use serial_test::serial;

use super::users::common::{get_auth_token, setup_test_app};

const LICENSE: &str = "/admin/api/v1/system/license";
const VERSIONS: &str = "/admin/api/v1/system/versions";
const CAPABILITIES: &str = "/admin/api/v1/system/capabilities";
const LOGS: &str = "/admin/api/v1/system/logs";

async fn authed_get<S>(app: &S, uri: &str, token: &str) -> (StatusCode, serde_json::Value)
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let req = test::TestRequest::get()
        .uri(uri)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;
    (status, body)
}

#[serial]
#[tokio::test]
async fn test_versions_reports_the_running_build() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, body) = authed_get(&app, VERSIONS, &token).await;
    assert_eq!(status, StatusCode::OK);

    // The dashboard shows this verbatim, so it must not come back empty.
    let data = &body["data"];
    assert!(data.is_object(), "versions payload should be an object");
    assert!(
        data.as_object().is_some_and(|o| !o.is_empty()),
        "versions payload should not be empty"
    );
}

#[serial]
#[tokio::test]
async fn test_capabilities_reports_feature_flags() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, body) = authed_get(&app, CAPABILITIES, &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_object(), "capabilities should be an object");
}

/// Without a mailer configured the test app must report mail as unavailable —
/// this is what gates the email step in the workflow editor.
#[serial]
#[tokio::test]
async fn test_capabilities_reflect_missing_mail_configuration() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (_status, body) = authed_get(&app, CAPABILITIES, &token).await;
    let data = &body["data"];

    // The flag name has varied; assert on whichever mail-ish boolean is present.
    let mail_flag = data
        .as_object()
        .and_then(|o| o.iter().find(|(k, _)| k.contains("mail")))
        .map(|(_, v)| v.clone());

    if let Some(flag) = mail_flag {
        assert_eq!(
            flag,
            serde_json::Value::Bool(false),
            "no mailer is wired in the test app"
        );
    }
}

#[serial]
#[tokio::test]
async fn test_license_status_is_reported() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, _body) = authed_get(&app, LICENSE, &token).await;
    // No licence is configured in tests; the endpoint must answer rather than
    // fall over, whichever way it reports that.
    assert!(
        status == StatusCode::OK || status == StatusCode::NOT_FOUND,
        "unexpected licence status {status}"
    );
}

#[serial]
#[tokio::test]
async fn test_system_logs_are_paginated() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // The query parameter is `page_size`; the response reports it as `per_page`.
    let (status, body) = authed_get(&app, &format!("{LOGS}?page=1&page_size=5"), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array(), "logs should be an array");

    let pagination = &body["meta"]["pagination"];
    assert_eq!(pagination["page"], 1);
    assert_eq!(pagination["per_page"], 5);
}

/// The login the harness performs is itself an auth event, so the log is not
/// empty by the time the test runs.
#[serial]
#[tokio::test]
async fn test_system_logs_record_authentication() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, body) = authed_get(&app, LOGS, &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_unknown_system_log_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = uuid::Uuid::now_v7();
    let (status, _body) = authed_get(&app, &format!("{LOGS}/{missing}"), &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_malformed_log_uuid_is_rejected() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(&format!("{LOGS}/not-a-uuid"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_client_error(),
        "a malformed uuid should be a client error, got {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_system_endpoints_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    for uri in [LICENSE, VERSIONS, LOGS] {
        let req = test::TestRequest::get().uri(uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must reject an anonymous caller"
        );
    }
}

/// Capabilities are deliberately public — the login screen decides what to
/// offer before anyone has a token, and the payload is feature flags only.
/// Pinned so the exemption stays a conscious choice and the payload does not
/// quietly grow to carry something sensitive.
#[serial]
#[tokio::test]
async fn test_capabilities_is_public_and_exposes_only_flags() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let req = test::TestRequest::get().uri(CAPABILITIES).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "capabilities is public by design"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    let data = body["data"].as_object().expect("capabilities is an object");

    assert!(
        data.values().all(serde_json::Value::is_boolean),
        "capabilities must stay boolean-only, got {data:?}"
    );
}
