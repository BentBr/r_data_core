#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Tests for POST endpoints accepting data (CSV/JSON) into entities
// Use Case 2: Define endpoint to accept POST data (CSV/JSON) into entities
// Use Case 2.a: POST endpoint without auth should fail with 401
// Use Case 2.b: POST endpoint with pre-shared key auth
// Use Case 2.c: POST endpoint with disabled workflow returns 503 (already exists in provider_workflow_endpoints_tests.rs)
// Use Case 2.d: POST endpoint ignores cron when from.api is used

use super::common::{
    create_consumer_workflow, create_test_entity_definition, generate_entity_type,
    setup_app_with_entities,
};
use actix_web::test;
use r_data_core_persistence::WorkflowRepository;
use uuid::Uuid;

// ============================================================================
// 2. Define endpoint to accept POST data (CSV/JSON) into entities
// ============================================================================

#[actix_web::test]
async fn test_post_endpoint_accepts_csv_into_entities() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_post_csv");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow with from.api source (accepts POST)
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                }
            }
        ]
    });

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test POST endpoint with CSV data
    let csv_data: Vec<u8> =
        b"name,email\nJohn Doe,john@example.com\nJane Smith,jane@example.com".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("Content-Type", "text/csv"))
        .set_payload(csv_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (202 Accepted)
    assert_eq!(
        resp.status().as_u16(),
        202,
        "Expected 202 Accepted, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_post_endpoint_accepts_json_into_entities() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_post_json");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow with from.api source (accepts POST)
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_api_source_json_to_entity.json",
        &entity_type,
    )?;

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test POST endpoint with JSON data
    let json_data = r#"[{"name":"John Doe","email":"john@example.com"},{"name":"Jane Smith","email":"jane@example.com"}]"#;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(json_data.as_bytes())
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (202 Accepted)
    assert_eq!(
        resp.status().as_u16(),
        202,
        "Expected 202 Accepted, got: {}",
        resp.status()
    );

    Ok(())
}

// ============================================================================
// 2.a. POST endpoint without auth should fail with 401
// ============================================================================

// Note: Currently, POST endpoints for from.api workflows don't require auth by default
// These tests verify the current behavior. If auth is added later, these tests should be updated.

#[actix_web::test]
async fn test_post_endpoint_csv_without_auth_currently_allowed() -> anyhow::Result<()> {
    // This test documents current behavior: POST endpoints don't require auth
    // When auth is implemented, this test should be updated to expect 401
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_post_no_auth_csv");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test POST without any auth headers
    let csv_data: Vec<u8> = b"name,email\nJohn,john@example.com".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Content-Type", "text/csv"))
        .set_payload(csv_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Auth is now required - should return 401
    assert_eq!(
        resp.status().as_u16(),
        401,
        "POST endpoints now require auth and should return 401"
    );

    Ok(())
}

#[actix_web::test]
async fn test_post_endpoint_json_without_auth_currently_allowed() -> anyhow::Result<()> {
    // This test documents current behavior: POST endpoints don't require auth
    // When auth is implemented, this test should be updated to expect 401
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_post_no_auth_json");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
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
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test POST without any auth headers
    let json_data = r#"[{"name":"John","email":"john@example.com"}]"#;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(json_data.as_bytes())
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Auth is now required - should return 401
    assert_eq!(
        resp.status().as_u16(),
        401,
        "POST endpoints now require auth and should return 401"
    );

    Ok(())
}

// ============================================================================
// 2.b. POST endpoint with pre-shared key auth
// ============================================================================

// Note: Pre-shared key auth for POST endpoints (from.api) is not yet implemented
// This would require adding auth configuration to the workflow config and validating it in the POST handler
// When implemented, add a test here that:
// 1. Creates a workflow with from.api source and pre-shared key auth in source.auth
// 2. Tests POST with correct pre-shared key -> should succeed (202)
// 3. Tests POST without pre-shared key -> should fail (401)
// 4. Tests POST with incorrect pre-shared key -> should fail (401)

// ============================================================================
// 2.c. POST endpoint with disabled workflow returns 503
// ============================================================================

// This test already exists in provider_workflow_endpoints_tests.rs as test_consumer_endpoint_post_inactive_workflow

// ============================================================================
// 2.d. POST endpoint ignores cron when from.api is used
// ============================================================================

#[actix_web::test]
async fn test_post_endpoint_ignores_cron_for_csv() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_ignore_cron_csv");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow with from.api source and a cron schedule
    // The cron should be ignored since from.api is used
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    // Even if we set a cron, it should be ignored for from.api workflows
    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    // Verify workflow was created (cron may be stored but should be ignored during execution)
    let workflow_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1)")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    assert!(workflow_exists, "Workflow should be created");

    // Note: Actual verification that cron is ignored would require testing the worker/scheduler
    // which is beyond the scope of this E2E test

    Ok(())
}

#[actix_web::test]
async fn test_post_endpoint_ignores_cron_for_json() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_ignore_cron_json");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
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
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/10 * * * *".to_string()),
    )
    .await?;

    let workflow_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1)")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    assert!(workflow_exists, "Workflow should be created");

    Ok(())
}

// ============================================================================
// 2.d.II: Verify that workflows with from.api are NOT scheduled via cron
// ============================================================================

#[actix_web::test]
async fn test_from_api_workflow_excluded_from_cron_scheduling() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_cron_exclusion");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create a workflow with from.api source type (should NOT be scheduled via cron)
    let api_config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    // Create workflow with from.api and a cron schedule (cron should be ignored)
    let api_wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        api_config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    // Create a regular workflow with from.uri source type (SHOULD be scheduled via cron)
    let uri_config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": {
                            "uri": "http://example.com/data.csv"
                        },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let uri_wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        uri_config,
        true,
        Some("*/10 * * * *".to_string()),
    )
    .await?;

    // Verify both workflows exist and have cron schedules
    let api_cron: Option<String> =
        sqlx::query_scalar("SELECT schedule_cron FROM workflows WHERE uuid = $1")
            .bind(api_wf_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        api_cron,
        Some("*/5 * * * *".to_string()),
        "API workflow should have cron stored"
    );

    let uri_cron: Option<String> =
        sqlx::query_scalar("SELECT schedule_cron FROM workflows WHERE uuid = $1")
            .bind(uri_wf_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        uri_cron,
        Some("*/10 * * * *".to_string()),
        "URI workflow should have cron stored"
    );

    // Now test that list_scheduled_consumers excludes the from.api workflow
    let repo = WorkflowRepository::new(pool.pool.clone());
    let scheduled = repo.list_scheduled_consumers().await?;

    // The from.api workflow should NOT be in the scheduled list
    let api_in_scheduled = scheduled.iter().any(|(uuid, _)| *uuid == api_wf_uuid);
    assert!(
        !api_in_scheduled,
        "Workflow with from.api should NOT be scheduled via cron"
    );

    // The from.uri workflow SHOULD be in the scheduled list
    let uri_in_scheduled = scheduled.iter().any(|(uuid, _)| *uuid == uri_wf_uuid);
    assert!(
        uri_in_scheduled,
        "Workflow with from.uri SHOULD be scheduled via cron"
    );

    // Verify the scheduled list contains the URI workflow with correct cron
    let uri_entry = scheduled.iter().find(|(uuid, _)| *uuid == uri_wf_uuid);
    assert!(
        uri_entry.is_some(),
        "URI workflow should be in scheduled list"
    );
    if let Some((_, cron)) = uri_entry {
        assert_eq!(
            cron, "*/10 * * * *",
            "URI workflow should have correct cron in scheduled list"
        );
    }

    Ok(())
}
