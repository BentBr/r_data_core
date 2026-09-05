#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! CRUD over `/admin/api/v1/roles`.
//!
//! Roles carry the permission grants every other authorisation check reads, so
//! the write paths matter: a duplicate name must not shadow an existing role,
//! and an update must actually replace the stored permission set.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::users::common::{get_auth_token, setup_test_app};

const BASE: &str = "/admin/api/v1/roles";

fn permission(resource: &str, permission_type: &str) -> serde_json::Value {
    serde_json::json!({
        "resource_type": resource,
        "permission_type": permission_type,
        "access_level": "All",
        "resource_uuids": [],
        "constraints": null,
    })
}

fn role_body(name: &str, permissions: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": format!("{name} role"),
        "super_admin": false,
        "permissions": permissions,
    })
}

async fn create<S>(app: &S, token: &str, name: &str) -> (StatusCode, serde_json::Value)
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
        .set_json(role_body(name, vec![permission("Entities", "Read")]))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;
    (status, body)
}

fn uuid_of(body: &serde_json::Value) -> String {
    body["data"]["uuid"]
        .as_str()
        .unwrap_or_else(|| panic!("no uuid in {body}"))
        .to_string()
}

#[serial]
#[tokio::test]
async fn test_create_then_read_round_trips() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, created) = create(&app, &token, "readers").await;
    assert!(status.is_success(), "create failed: {status} {created}");
    let uuid = uuid_of(&created);

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["name"], "readers");
    assert_eq!(
        body["data"]["permissions"].as_array().map(Vec::len),
        Some(1),
        "the created permission must come back"
    );
}

/// Role names are how permissions are reasoned about, so a duplicate has to be
/// refused rather than creating a second role nobody can tell apart.
#[serial]
#[tokio::test]
async fn test_duplicate_role_name_is_rejected() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (first, _) = create(&app, &token, "duplicated").await;
    assert!(first.is_success());

    let (second, _) = create(&app, &token, "duplicated").await;
    assert_eq!(second, StatusCode::CONFLICT);
}

#[serial]
#[tokio::test]
async fn test_list_includes_created_roles() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    create(&app, &token, "listed-alpha").await;
    create(&app, &token, "listed-beta").await;

    let req = test::TestRequest::get()
        .uri(BASE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let names: Vec<String> = body["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();

    assert!(names.iter().any(|n| n == "listed-alpha"), "got {names:?}");
    assert!(names.iter().any(|n| n == "listed-beta"), "got {names:?}");
}

#[serial]
#[tokio::test]
async fn test_update_replaces_the_permission_set() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (_, created) = create(&app, &token, "mutable").await;
    let uuid = uuid_of(&created);

    let req = test::TestRequest::put()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(role_body(
            "mutable",
            vec![
                permission("Entities", "Read"),
                permission("Workflows", "Execute"),
            ],
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "update failed: {}",
        resp.status()
    );

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(
        body["data"]["permissions"].as_array().map(Vec::len),
        Some(2),
        "the new permission set must replace the old one"
    );
}

#[serial]
#[tokio::test]
async fn test_delete_removes_the_role() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (_, created) = create(&app, &token, "disposable").await;
    let uuid = uuid_of(&created);

    let req = test::TestRequest::delete()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "delete failed: {}",
        resp.status()
    );

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_unknown_role_is_not_found() {
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

#[serial]
#[tokio::test]
async fn test_role_routes_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = Uuid::now_v7();

    let anonymous = [
        test::TestRequest::get().uri(BASE).to_request(),
        test::TestRequest::get()
            .uri(&format!("{BASE}/{missing}"))
            .to_request(),
        test::TestRequest::post()
            .uri(BASE)
            .set_json(role_body("anon", vec![]))
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
