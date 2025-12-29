#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Tests for workflow route conflicts
// Verifies that workflow routes are matched correctly and don't conflict with dynamic entity routes

use super::common::{create_provider_workflow, setup_app_with_entities};
use actix_web::test;
use uuid::Uuid;

// ============================================================================
// Route conflict tests
// ============================================================================

/// Test that GET /api/v1/workflows/{uuid} routes to workflow handler, not dynamic entity handler
/// This test ensures the route order fix works correctly
#[actix_web::test]
async fn test_get_workflow_routes_to_workflow_handler_not_entity() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create a provider workflow with entity source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": "test_entity",
                    "filter": null,
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, config).await?;

    // Test GET /api/v1/workflows/{uuid} - should route to workflow handler
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;

    // The workflow handler should return data or error, but NOT the dynamic entity "not found" error
    // If it routes to dynamic entity handler, we'd get:
    // "Entity of type 'workflows' with UUID '...' not found"
    if status.as_u16() == 404 {
        let error_msg = body["message"].as_str().unwrap_or("");
        assert!(
            !error_msg.contains("Entity of type"),
            "Route conflict detected! Request was routed to dynamic entity handler instead of workflow handler. Error: {error_msg}"
        );
    }

    // Workflow handler should return success or 500 (if entity definition doesn't exist)
    // But NOT 404 with dynamic entity error
    assert!(
        status.is_success() || status.as_u16() == 500 || status.as_u16() == 404,
        "Unexpected status: {status}"
    );

    Ok(())
}

/// Test that GET /api/v1/workflows/{uuid}/trigger routes correctly
#[actix_web::test]
async fn test_get_workflow_trigger_routes_correctly() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create a provider workflow (trigger endpoint expects consumer with trigger source,
    // so provider should return 404 with specific message)
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": "test_entity",
                    "filter": null,
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, config).await?;

    // Test GET /api/v1/workflows/{uuid}/trigger - should route to trigger handler
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}/trigger"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should return 404 because it's a Provider workflow (trigger expects Consumer)
    // The error message should be from trigger handler, not dynamic entity handler
    assert_eq!(
        status.as_u16(),
        404,
        "Expected 404 for Provider workflow on trigger endpoint"
    );

    let error_msg = body["message"].as_str().unwrap_or("");
    // Should NOT contain dynamic entity error
    assert!(
        !error_msg.contains("Entity of type"),
        "Route conflict detected! Request was routed to dynamic entity handler. Error: {error_msg}"
    );

    Ok(())
}

/// Test that GET /api/v1/workflows/{uuid}/stats routes correctly
#[actix_web::test]
async fn test_get_workflow_stats_routes_correctly() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create a provider workflow
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": "test_entity",
                    "filter": null,
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, config).await?;

    // Test GET /api/v1/workflows/{uuid}/stats - should route to stats handler
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}/stats"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Stats endpoint should return metadata about the workflow
    if status.as_u16() == 404 {
        let error_msg = body["message"].as_str().unwrap_or("");
        assert!(
            !error_msg.contains("Entity of type"),
            "Route conflict detected! Request was routed to dynamic entity handler. Error: {error_msg}"
        );
    }

    // Should return success (stats work even if entity definition doesn't exist) or 400 (bad config)
    assert!(
        status.is_success() || status.as_u16() == 400,
        "Expected success or 400 for stats endpoint, got: {status}"
    );

    // Verify it's the stats response (should have uuid and name fields)
    if status.is_success() {
        assert!(
            body.get("uuid").is_some(),
            "Stats response should contain 'uuid' field"
        );
        assert!(
            body.get("name").is_some(),
            "Stats response should contain 'name' field"
        );
    }

    Ok(())
}

/// Test that workflow routes work even when entity type "workflows" exists
/// This is an edge case where a user might create an entity definition called "workflows"
#[actix_web::test]
async fn test_workflow_routes_work_even_with_workflows_entity_type() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Note: We can't easily create an entity type called "workflows" in tests
    // because the entity type naming might have restrictions
    // But we can at least verify that the workflow routes work correctly

    // Create a provider workflow
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": {
                            "uri": "http://example.com/data.json"
                        },
                        "auth": null
                    },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_provider_workflow(&pool.pool, creator_uuid, config).await?;

    // Test GET /api/v1/workflows/{uuid} - should route to workflow handler
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should route to workflow handler (success or 500 from external URI failure)
    // NOT to dynamic entity handler
    if status.as_u16() == 404 {
        let error_msg = body["message"].as_str().unwrap_or("");
        assert!(
            !error_msg.contains("Entity of type 'workflows'"),
            "Route conflict! Routed to dynamic entity handler instead of workflow handler"
        );
    }

    Ok(())
}

/// Test that non-existent workflow returns proper error from workflow handler
#[actix_web::test]
async fn test_nonexistent_workflow_returns_workflow_error() -> anyhow::Result<()> {
    let (app, _pool, token, _) = setup_app_with_entities().await?;

    let fake_uuid = Uuid::now_v7();

    // Test GET /api/v1/workflows/{uuid} with non-existent UUID
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{fake_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should return 404 from workflow handler
    assert_eq!(
        status.as_u16(),
        404,
        "Expected 404 for non-existent workflow"
    );

    // Error should be "Workflow not found", not "Entity of type 'workflows' not found"
    let error_msg = body["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("Workflow not found") || error_msg.contains("not found"),
        "Expected workflow error message, got: {body}"
    );
    assert!(
        !error_msg.contains("Entity of type"),
        "Route conflict! Got dynamic entity error instead of workflow error"
    );

    Ok(())
}
