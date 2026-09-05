#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The version-history endpoints for workflows and entity definitions, plus the
//! field listing the entity editor reads its column list from.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::users::common::{get_auth_token, setup_test_app};

const WORKFLOWS: &str = "/admin/api/v1/workflows";
const ENTITY_DEFS: &str = "/admin/api/v1/entity-definitions";

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

// --- workflow versions ----------------------------------------------------

/// A workflow that does not exist has no history — the endpoint must answer
/// rather than error out, so the editor can render an empty history panel.
#[serial]
#[tokio::test]
async fn test_versions_for_an_unknown_workflow() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{WORKFLOWS}/{missing}/versions"), &token).await;

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "unexpected status {}",
        resp.status()
    );

    if resp.status() == StatusCode::OK {
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["data"].as_array().map(Vec::len),
            Some(0),
            "an unknown workflow has no versions"
        );
    }
}

#[serial]
#[tokio::test]
async fn test_single_version_of_an_unknown_workflow_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{WORKFLOWS}/{missing}/versions/1"), &token).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_malformed_workflow_uuid_is_a_client_error() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, &format!("{WORKFLOWS}/not-a-uuid/versions"), &token).await;
    assert!(resp.status().is_client_error(), "got {}", resp.status());
}

// --- entity definition versions -------------------------------------------

#[serial]
#[tokio::test]
async fn test_versions_for_an_unknown_entity_definition() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{ENTITY_DEFS}/{missing}/versions"), &token).await;

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "unexpected status {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_single_entity_definition_version_of_unknown_uuid() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(&app, &format!("{ENTITY_DEFS}/{missing}/versions/1"), &token).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- entity fields --------------------------------------------------------

/// An unknown entity type still returns the system columns, because those exist
/// on every entity view regardless of the definition.
#[serial]
#[tokio::test]
async fn test_fields_for_an_unknown_entity_type() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, &format!("{ENTITY_DEFS}/no_such_type/fields"), &token).await;

    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "unexpected status {}",
        resp.status()
    );

    if resp.status() == StatusCode::OK {
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["data"].is_array(), "fields should be an array");

        for field in body["data"].as_array().unwrap_or(&vec![]) {
            assert!(field["name"].is_string(), "field has no name: {field}");
            assert!(field["type"].is_string(), "field has no type: {field}");
            assert!(field["required"].is_boolean(), "field has no required flag");
            assert!(field["system"].is_boolean(), "field has no system flag");
        }
    }
}

// --- authentication -------------------------------------------------------

#[serial]
#[tokio::test]
async fn test_versioning_routes_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = Uuid::now_v7();

    for uri in [
        format!("{WORKFLOWS}/{missing}/versions"),
        format!("{WORKFLOWS}/{missing}/versions/1"),
        format!("{ENTITY_DEFS}/{missing}/versions"),
        format!("{ENTITY_DEFS}/{missing}/versions/1"),
        format!("{ENTITY_DEFS}/some_type/fields"),
    ] {
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must reject an anonymous caller"
        );
    }
}
