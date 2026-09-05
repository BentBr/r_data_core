#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The workflow listing endpoints the admin dashboard pages through: the
//! workflow list, the global run list, per-workflow runs, and run logs.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::users::common::{get_auth_token, setup_test_app};

const WORKFLOWS: &str = "/admin/api/v1/workflows";
const ALL_RUNS: &str = "/admin/api/v1/workflows/runs";

async fn get_authed<S>(app: &S, uri: &str, token: &str) -> actix_web::dev::ServiceResponse
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
    test::call_service(app, req).await
}

#[serial]
#[tokio::test]
async fn test_workflow_list_is_paginated() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, &format!("{WORKFLOWS}?page=1&per_page=5"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array(), "workflows should be an array");

    let pagination = &body["meta"]["pagination"];
    assert_eq!(pagination["page"], 1);
    assert_eq!(pagination["per_page"], 5);
}

/// An empty database must still answer with an empty page rather than an error,
/// otherwise a fresh install shows a broken dashboard.
#[serial]
#[tokio::test]
async fn test_workflow_list_is_empty_on_a_fresh_database() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, WORKFLOWS, &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().map(Vec::len), Some(0));
    assert_eq!(body["meta"]["pagination"]["total"], 0);
}

#[serial]
#[tokio::test]
async fn test_global_run_list_answers_on_an_empty_database() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, ALL_RUNS, &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
}

#[serial]
#[tokio::test]
async fn test_runs_for_an_unknown_workflow() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{WORKFLOWS}/{missing}/runs"), &token).await;

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "unexpected status {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_logs_for_an_unknown_run() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{ALL_RUNS}/{missing}/logs"), &token).await;

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "unexpected status {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_running_an_unknown_workflow_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let req = test::TestRequest::post()
        .uri(&format!("{WORKFLOWS}/{missing}/run"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_client_error(),
        "running a missing workflow should be a client error, got {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_malformed_pagination_is_handled() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // A non-numeric page must not 500; either it is rejected or defaulted.
    let resp = get_authed(&app, &format!("{WORKFLOWS}?page=abc"), &token).await;
    assert!(
        resp.status() == StatusCode::OK || resp.status().is_client_error(),
        "got {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_workflow_listing_requires_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = Uuid::now_v7();

    for uri in [
        WORKFLOWS.to_string(),
        ALL_RUNS.to_string(),
        format!("{WORKFLOWS}/{missing}/runs"),
        format!("{ALL_RUNS}/{missing}/logs"),
    ] {
        let resp = test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must reject an anonymous caller"
        );
    }
}
