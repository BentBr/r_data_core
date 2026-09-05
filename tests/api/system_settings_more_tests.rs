#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The entity-versioning and workflow-run-log settings endpoints.
//!
//! Both are partial-update PUTs merged over the stored values, so the tests
//! check that an omitted field is left alone rather than reset to a default —
//! the failure mode that silently disables retention.

use actix_web::{http::StatusCode, test};
use serial_test::serial;

use super::users::common::{get_auth_token, setup_test_app};

const VERSIONING: &str = "/admin/api/v1/system/settings/entity-versioning";
const RUN_LOGS: &str = "/admin/api/v1/system/settings/workflow-run-logs";

async fn get_settings<S>(app: &S, uri: &str, token: &str) -> (StatusCode, serde_json::Value)
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

async fn put_settings<S>(app: &S, uri: &str, token: &str, body: serde_json::Value) -> StatusCode
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let req = test::TestRequest::put()
        .uri(uri)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(body)
        .to_request();
    test::call_service(app, req).await.status()
}

#[serial]
#[tokio::test]
async fn test_versioning_settings_have_defaults() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, body) = get_settings(&app, VERSIONING, &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["data"]["enabled"].is_boolean(),
        "settings must report an enabled flag: {body}"
    );
}

#[serial]
#[tokio::test]
async fn test_versioning_settings_round_trip() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let status = put_settings(
        &app,
        VERSIONING,
        &token,
        serde_json::json!({ "enabled": true, "max_versions": 7, "max_age_days": 30 }),
    )
    .await;
    assert!(status.is_success(), "update failed: {status}");

    let (_, body) = get_settings(&app, VERSIONING, &token).await;
    assert_eq!(body["data"]["enabled"], true);
    assert_eq!(body["data"]["max_versions"], 7);
    assert_eq!(body["data"]["max_age_days"], 30);
}

/// A partial update must merge, not reset. Sending only `enabled` used to be
/// the obvious way to toggle versioning; if that wiped the retention limits,
/// history would grow without bound.
#[serial]
#[tokio::test]
async fn test_versioning_partial_update_preserves_other_fields() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    put_settings(
        &app,
        VERSIONING,
        &token,
        serde_json::json!({ "enabled": true, "max_versions": 5, "max_age_days": 14 }),
    )
    .await;

    let status = put_settings(
        &app,
        VERSIONING,
        &token,
        serde_json::json!({ "enabled": false }),
    )
    .await;
    assert!(status.is_success());

    let (_, body) = get_settings(&app, VERSIONING, &token).await;
    assert_eq!(body["data"]["enabled"], false, "the toggle applied");
    assert_eq!(
        body["data"]["max_versions"], 5,
        "an omitted field must survive the partial update"
    );
    assert_eq!(body["data"]["max_age_days"], 14);
}

#[serial]
#[tokio::test]
async fn test_run_log_settings_round_trip() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, _) = get_settings(&app, RUN_LOGS, &token).await;
    assert_eq!(status, StatusCode::OK);

    let status = put_settings(
        &app,
        RUN_LOGS,
        &token,
        serde_json::json!({ "enabled": true, "max_runs": 25, "max_age_days": 3 }),
    )
    .await;
    assert!(status.is_success(), "update failed: {status}");

    let (_, body) = get_settings(&app, RUN_LOGS, &token).await;
    assert_eq!(body["data"]["enabled"], true);
    assert_eq!(body["data"]["max_runs"], 25);
    assert_eq!(body["data"]["max_age_days"], 3);
}

#[serial]
#[tokio::test]
async fn test_run_log_partial_update_preserves_other_fields() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    put_settings(
        &app,
        RUN_LOGS,
        &token,
        serde_json::json!({ "enabled": true, "max_runs": 11, "max_age_days": 9 }),
    )
    .await;

    put_settings(
        &app,
        RUN_LOGS,
        &token,
        serde_json::json!({ "enabled": false }),
    )
    .await;

    let (_, body) = get_settings(&app, RUN_LOGS, &token).await;
    assert_eq!(body["data"]["enabled"], false);
    assert_eq!(body["data"]["max_runs"], 11);
    assert_eq!(body["data"]["max_age_days"], 9);
}

#[serial]
#[tokio::test]
async fn test_settings_endpoints_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    for uri in [VERSIONING, RUN_LOGS] {
        let resp = test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{uri} GET");

        let req = test::TestRequest::put()
            .uri(uri)
            .set_json(serde_json::json!({ "enabled": true }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{uri} PUT");
    }
}
