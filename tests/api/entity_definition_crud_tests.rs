#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! CRUD over `/admin/api/v1/entity-definitions`, plus the password-reset route
//! branches that fire when no mailer is wired.
//!
//! An entity definition is the schema every dynamic entity table is generated
//! from, so the rejection paths matter more than the happy one: a bad entity
//! type or a duplicate would otherwise reach DDL generation.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::users::common::{get_auth_token, setup_test_app};

const BASE: &str = "/admin/api/v1/entity-definitions";
const FORGOT: &str = "/admin/api/v1/auth/forgot-password";
const RESET: &str = "/admin/api/v1/auth/reset-password";

fn definition(entity_type: &str) -> serde_json::Value {
    serde_json::json!({
        "entity_type": entity_type,
        "display_name": "Test Definition",
        "description": "created by the integration suite",
        "group_name": "tests",
        "allow_children": false,
        "icon": "box",
        "fields": [{
            "name": "title",
            "display_name": "Title",
            "field_type": "String",
            "required": true,
            "indexed": false,
            "filterable": true,
            "description": null,
            "default_value": null,
            "constraints": {},
            "ui_settings": {}
        }],
        "published": true,
    })
}

async fn create<S>(app: &S, token: &str, entity_type: &str) -> (StatusCode, serde_json::Value)
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let req = test::TestRequest::post()
        .uri(BASE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(definition(entity_type))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;
    (status, body)
}

#[serial]
#[tokio::test]
async fn test_list_is_paginated_on_a_fresh_database() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}?page=1&per_page=5"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());
    assert_eq!(body["meta"]["pagination"]["per_page"], 5);
}

#[serial]
#[tokio::test]
async fn test_unknown_definition_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{missing}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// The entity type becomes a table name, so anything that is not a plain
/// identifier has to be refused before it reaches DDL generation.
#[serial]
#[tokio::test]
async fn test_entity_type_with_sql_metacharacters_is_refused() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    for bad in [
        "drop; DROP TABLE admin_users; --",
        "has space",
        "has-dash",
        "",
    ] {
        let (status, body) = create(&app, &token, bad).await;
        assert!(
            status.is_client_error(),
            "entity type {bad:?} must be refused, got {status} {body}"
        );
    }

    // The table the injection targets must still be there.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_users")
        .fetch_one(&pool.pool)
        .await
        .expect("admin_users must still exist");
    assert!(count > 0);
}

#[serial]
#[tokio::test]
async fn test_deleting_an_unknown_definition_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let req = test::TestRequest::delete()
        .uri(&format!("{BASE}/{missing}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_updating_an_unknown_definition_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = Uuid::now_v7();
    let req = test::TestRequest::put()
        .uri(&format!("{BASE}/{missing}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(definition("some_type"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_entity_definition_routes_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = Uuid::now_v7();

    let anonymous = [
        test::TestRequest::get().uri(BASE).to_request(),
        test::TestRequest::get()
            .uri(&format!("{BASE}/{missing}"))
            .to_request(),
        test::TestRequest::post()
            .uri(BASE)
            .set_json(definition("anon_type"))
            .to_request(),
        test::TestRequest::delete()
            .uri(&format!("{BASE}/{missing}"))
            .to_request(),
    ];

    for req in anonymous {
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

// --- password reset with no mailer wired ----------------------------------

/// With no mailer configured the endpoint must still answer with the same
/// generic message, so an attacker cannot tell a misconfigured deployment from
/// a missing account.
#[serial]
#[tokio::test]
async fn test_forgot_password_is_generic_without_a_mailer() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let mut bodies = Vec::new();
    for email in ["nobody@example.com", "admin@example.com"] {
        let req = test::TestRequest::post()
            .uri(FORGOT)
            .set_json(serde_json::json!({ "email": email }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let mut body: serde_json::Value = test::read_body_json(resp).await;
        if let Some(meta) = body.get_mut("meta") {
            *meta = serde_json::Value::Null;
        }
        bodies.push(body);
    }

    assert_eq!(
        bodies[0], bodies[1],
        "the response must not reveal whether the account exists"
    );
}

#[serial]
#[tokio::test]
async fn test_reset_password_reports_the_feature_is_unavailable() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let req = test::TestRequest::post()
        .uri(RESET)
        .set_json(serde_json::json!({
            "token": "whatever",
            "new_password": "a_long_enough_password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "with no mailer wired the reset flow is unavailable"
    );
}

/// Neither endpoint may require a token — a locked-out user has no session.
#[serial]
#[tokio::test]
async fn test_password_reset_endpoints_are_anonymous() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    for (uri, body) in [
        (FORGOT, serde_json::json!({ "email": "x@example.com" })),
        (
            RESET,
            serde_json::json!({ "token": "t", "new_password": "a_long_enough_password" }),
        ),
    ] {
        let req = test::TestRequest::post()
            .uri(uri)
            .set_json(body)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_ne!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must be reachable without a session"
        );
    }
}
