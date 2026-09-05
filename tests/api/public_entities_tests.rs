#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The public `/api/v1/entities*` surface.
//!
//! These are the read endpoints external consumers hit, so the authorisation
//! checks matter as much as the payloads: they accept either a JWT or an API
//! key, and must refuse an anonymous caller.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::users::common::{get_auth_token, setup_test_app};

const ENTITIES: &str = "/api/v1/entities";
const BY_PATH: &str = "/api/v1/entities/by-path";
const QUERY: &str = "/api/v1/entities/query";

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
async fn test_listing_entity_types_on_a_fresh_database() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, ENTITIES, &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body.is_array() || body["data"].is_array(),
        "entity types should come back as a list: {body}"
    );
}

/// `path` is optional and defaults to the root, so the browser can open on
/// `/` without constructing a query string.
#[serial]
#[tokio::test]
async fn test_by_path_defaults_to_the_root() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, BY_PATH, &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["data"].is_array(),
        "browse should return a list: {body}"
    );
}

/// `limit` is clamped rather than rejected, so a caller cannot ask for an
/// unbounded page.
#[serial]
#[tokio::test]
async fn test_by_path_clamps_an_oversized_limit() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, &format!("{BY_PATH}?path=/&limit=100000"), &token).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let per_page = body["meta"]["pagination"]["per_page"].as_i64().unwrap_or(0);
    assert!(
        (1..=100).contains(&per_page),
        "limit should be clamped into 1..=100, got {per_page}"
    );
}

#[serial]
#[tokio::test]
async fn test_by_path_with_an_unknown_path() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = get_authed(&app, &format!("{BY_PATH}?path=/nothing/here"), &token).await;
    assert!(
        resp.status() == StatusCode::OK || resp.status().is_client_error(),
        "unexpected status {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_versions_of_an_unknown_entity() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(
        &app,
        &format!("{ENTITIES}/no_such_type/{missing}/versions"),
        &token,
    )
    .await;

    assert!(
        !resp.status().is_server_error(),
        "an unknown entity type must not 500, got {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_single_version_of_an_unknown_entity() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let resp = get_authed(
        &app,
        &format!("{ENTITIES}/no_such_type/{missing}/versions/1"),
        &token,
    )
    .await;

    assert!(
        !resp.status().is_server_error(),
        "an unknown version must not 500, got {}",
        resp.status()
    );
}

/// An injected filter key against an unknown entity type is answered without
/// error and without reaching the query builder — the lookup short-circuits on
/// the missing definition first. The identifier validator that guards the real
/// path is covered by the persistence unit tests and by
/// `query_validation_integration_tests`; what matters here is that the caller's
/// string never turns into a server error or a executed statement.
#[serial]
#[tokio::test]
async fn test_injected_filter_key_is_answered_harmlessly() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(QUERY)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "entity_type": "no_such_type",
            "filters": { "uuid; DROP TABLE admin_users; --": "x" }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        !resp.status().is_server_error(),
        "an injected filter key must not reach the query builder, got {}",
        resp.status()
    );

    // The table the injection targets must still be there.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_users")
        .fetch_one(&pool.pool)
        .await
        .expect("admin_users must still exist");
    assert!(count > 0, "the seeded admin user must survive");
}

#[serial]
#[tokio::test]
async fn test_unknown_entity_type_is_not_a_server_error() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(QUERY)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "entity_type": "definitely_not_a_type" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(
        !resp.status().is_server_error(),
        "an unknown entity type must not be a server error, got {}",
        resp.status()
    );
}

#[serial]
#[tokio::test]
async fn test_public_entity_routes_reject_anonymous_callers() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = Uuid::now_v7();

    for uri in [
        ENTITIES.to_string(),
        format!("{BY_PATH}?path=/x"),
        format!("{ENTITIES}/some_type/{missing}/versions"),
    ] {
        let resp = test::call_service(&app, test::TestRequest::get().uri(&uri).to_request()).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must reject an anonymous caller"
        );
    }

    let req = test::TestRequest::post()
        .uri(QUERY)
        .set_json(serde_json::json!({ "entity_type": "x" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// A bearer token that is not a real JWT must be refused, not treated as
/// anonymous-but-allowed.
#[serial]
#[tokio::test]
async fn test_public_entity_routes_reject_a_bogus_token() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let resp = get_authed(&app, ENTITIES, "not.a.jwt").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
