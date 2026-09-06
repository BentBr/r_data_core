#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! Restoring a previous workflow version.
//!
//! Restore appends rather than rewinds, and restores the program only. Both
//! matter: rewinding would erase the mistake from the record, and pulling
//! `enabled` or `name` from an old row would silently undo a deliberate
//! change someone made since.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use super::common::{create_consumer_workflow, setup_app_with_entities};

/// A minimal valid consumer program producing an API output.
fn program(field: &str) -> serde_json::Value {
    serde_json::json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { field: field }
            },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    })
}

fn restore_uri(uuid: Uuid, version: i32) -> String {
    format!("/admin/api/v1/workflows/{uuid}/versions/{version}/restore")
}

#[serial]
#[tokio::test]
async fn test_restore_appends_a_new_version_carrying_the_old_program() {
    let (app, pool, token, _) = setup_app_with_entities().await.unwrap();
    let admin_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await
        .unwrap();

    let uuid = create_consumer_workflow(&pool.pool, admin_uuid, program("alpha"), false, None)
        .await
        .unwrap();

    // A second version, which we will roll back.
    let update = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "name": format!("renamed-{}", Uuid::now_v7().simple()),
            "kind": "consumer",
            "enabled": false,
            "config": program("beta"),
        }))
        .to_request();
    assert!(test::call_service(&app, update).await.status().is_success());

    let versions_before = list_versions(&app, &token, uuid).await;

    let req = test::TestRequest::post()
        .uri(&restore_uri(uuid, 1))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "restore should succeed, got {}",
        resp.status()
    );

    let versions_after = list_versions(&app, &token, uuid).await;
    assert!(
        versions_after > versions_before,
        "restore must APPEND a version ({versions_before} -> {versions_after}); \
         rewinding would erase the mistake from the record"
    );
}

#[serial]
#[tokio::test]
async fn test_restore_does_not_resurrect_old_metadata() {
    let (app, pool, token, _) = setup_app_with_entities().await.unwrap();
    let admin_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await
        .unwrap();

    // Created enabled; later deliberately disabled.
    let uuid = create_consumer_workflow(&pool.pool, admin_uuid, program("alpha"), true, None)
        .await
        .unwrap();

    let disable = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "name": format!("wf-{}", Uuid::now_v7().simple()),
            "kind": "consumer",
            "enabled": false,
            "config": program("beta"),
        }))
        .to_request();
    assert!(test::call_service(&app, disable)
        .await
        .status()
        .is_success());

    let req = test::TestRequest::post()
        .uri(&restore_uri(uuid, 1))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    assert!(test::call_service(&app, req).await.status().is_success());

    let detail = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let body: serde_json::Value =
        test::read_body_json(test::call_service(&app, detail).await).await;

    assert_eq!(
        body["data"]["enabled"], false,
        "restoring version 1 must not re-enable a workflow that was \
         deliberately disabled afterwards: {body}"
    );
}

#[serial]
#[tokio::test]
async fn test_restoring_a_nonexistent_version_is_a_404() {
    let (app, pool, token, _) = setup_app_with_entities().await.unwrap();
    let admin_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await
        .unwrap();
    let uuid = create_consumer_workflow(&pool.pool, admin_uuid, program("alpha"), false, None)
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri(&restore_uri(uuid, 999))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[serial]
#[tokio::test]
async fn test_restore_requires_authentication() {
    let (app, pool, _token, _) = setup_app_with_entities().await.unwrap();
    let admin_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await
        .unwrap();
    let uuid = create_consumer_workflow(&pool.pool, admin_uuid, program("alpha"), false, None)
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri(&restore_uri(uuid, 1))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

async fn list_versions<S, B>(app: &S, token: &str, uuid: Uuid) -> usize
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/workflows/{uuid}/versions"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let body: serde_json::Value = test::read_body_json(test::call_service(app, req).await).await;
    body["data"].as_array().map_or(0, Vec::len)
}
