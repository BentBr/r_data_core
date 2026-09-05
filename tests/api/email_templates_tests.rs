#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! CRUD over `/admin/api/v1/email-templates`.

use actix_web::{http::StatusCode, test};
use serial_test::serial;

use super::users::common::{get_auth_token, setup_test_app};

const BASE: &str = "/admin/api/v1/email-templates";

fn payload(slug: &str) -> serde_json::Value {
    serde_json::json!({
        "name": format!("Template {slug}"),
        "slug": slug,
        "subject_template": "Hello {{ name }}",
        "body_html_template": "<p>Hello {{ name }}</p>",
        "body_text_template": "Hello {{ name }}",
        "variables": ["name"],
    })
}

async fn create<S>(app: &S, token: &str, slug: &str) -> (StatusCode, serde_json::Value)
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
        .set_json(payload(slug))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;
    (status, body)
}

fn uuid_of(body: &serde_json::Value) -> String {
    body["data"]["uuid"]
        .as_str()
        .or_else(|| body["data"].as_str())
        .unwrap_or_else(|| panic!("no uuid in {body}"))
        .to_string()
}

#[serial]
#[tokio::test]
async fn test_create_then_read_round_trips() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (status, body) = create(&app, &token, "welcome").await;
    assert!(status.is_success(), "create failed: {status} {body}");
    let uuid = uuid_of(&body);

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let fetched: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(fetched["data"]["slug"], "welcome");
    assert_eq!(fetched["data"]["subject_template"], "Hello {{ name }}");
}

/// The slug is the lookup key the workflow engine resolves templates by, so a
/// duplicate has to be refused rather than silently shadowing the first.
#[serial]
#[tokio::test]
async fn test_duplicate_slug_is_rejected() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (first, _) = create(&app, &token, "duplicate").await;
    assert!(first.is_success());

    let (second, _) = create(&app, &token, "duplicate").await;
    assert_eq!(second, StatusCode::CONFLICT);
}

#[serial]
#[tokio::test]
async fn test_list_returns_created_templates() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    create(&app, &token, "listed-one").await;
    create(&app, &token, "listed-two").await;

    let req = test::TestRequest::get()
        .uri(BASE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let slugs: Vec<String> = body["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| t["slug"].as_str().map(str::to_string))
        .collect();

    assert!(slugs.iter().any(|s| s == "listed-one"), "got {slugs:?}");
    assert!(slugs.iter().any(|s| s == "listed-two"), "got {slugs:?}");
}

#[serial]
#[tokio::test]
async fn test_update_changes_the_stored_template() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let (_, created) = create(&app, &token, "editable").await;
    let uuid = uuid_of(&created);

    let req = test::TestRequest::put()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        // PUT is a full replace: every body field but `name` is required.
        .set_json(serde_json::json!({
            "name": "Renamed",
            "subject_template": "Updated subject",
            "body_html_template": "<p>Updated</p>",
            "body_text_template": "Updated",
            "variables": ["name"],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "update failed with {}",
        resp.status()
    );

    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body["data"]["subject_template"], "Updated subject");
}

#[serial]
#[tokio::test]
async fn test_delete_removes_the_template() {
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
async fn test_unknown_template_is_not_found() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let missing = uuid::Uuid::now_v7();
    let req = test::TestRequest::get()
        .uri(&format!("{BASE}/{missing}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn test_email_template_routes_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();
    let missing = uuid::Uuid::now_v7();

    let anonymous = [
        test::TestRequest::get().uri(BASE).to_request(),
        test::TestRequest::get()
            .uri(&format!("{BASE}/{missing}"))
            .to_request(),
        test::TestRequest::post()
            .uri(BASE)
            .set_json(payload("anon"))
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
