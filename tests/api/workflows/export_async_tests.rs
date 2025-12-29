#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::test;
use serde_json::json;
use uuid::Uuid;

use crate::api::workflows::common::{
    create_entity_definition_with_fields, create_provider_workflow, generate_entity_type,
    setup_app_with_entities,
};

#[actix_web::test]
async fn test_provider_get_async_enqueues_run() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("async_api");

    // Minimal entity definition
    create_entity_definition_with_fields(&pool.pool, &entity_type, vec![]).await?;

    // Load provider workflow that outputs API JSON
    let cfg = crate::api::workflows::common::load_workflow_example(
        "workflow_format_to_api_json.json",
        &entity_type,
    )?;

    // Create workflow
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;
    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, cfg).await?;

    // First call with async=true should enqueue and return queued
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}?async=true"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 202);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let run_uuid = body["run_uuid"].as_str().expect("run_uuid missing");
    assert_eq!(body["status"], json!("queued"));

    // Poll the same run_uuid; should be queued or running
    let req2 = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/workflows/{wf_uuid}?async=true&run_uuid={run_uuid}"
        ))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 200);
    let body2: serde_json::Value = test::read_body_json(resp2).await;
    let status = body2["status"].as_str().unwrap_or("");
    assert!(status == "queued" || status == "running" || status == "success");
    Ok(())
}

#[actix_web::test]
async fn test_admin_run_now_returns_queued() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("admin_run");

    create_entity_definition_with_fields(&pool.pool, &entity_type, vec![]).await?;

    let cfg = crate::api::workflows::common::load_workflow_example(
        "workflow_format_to_api_json.json",
        &entity_type,
    )?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;
    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, cfg).await?;

    let req = test::TestRequest::post()
        .uri(&format!("/admin/api/v1/workflows/{wf_uuid}/run"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    // We accept either explicit queued status or a success wrapper, but run_uuid must exist
    let data = body.get("data").unwrap_or(&body);
    let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let enqueued_msg = data
        .get("message")
        .and_then(|v| v.as_str())
        .is_some_and(|m| m.to_lowercase().contains("enqueued"));
    assert!(
        status.eq_ignore_ascii_case("queued")
            || status.eq_ignore_ascii_case("success")
            || enqueued_msg,
        "Expected queued/success/enqueued status, got: {body:?}"
    );
    assert!(
        data["run_uuid"].as_str().is_some(),
        "run_uuid missing in response: {body}"
    );
    Ok(())
}
