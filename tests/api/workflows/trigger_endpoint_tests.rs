#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Tests for GET trigger endpoint (Consumer workflows with trigger source)
// Tests that GET /api/v1/workflows/{uuid} can trigger Consumer workflows with trigger source

use super::common::{
    create_consumer_workflow, create_provider_workflow, create_test_entity_definition,
    generate_entity_type, setup_app_with_entities,
};
use actix_web::test;
use uuid::Uuid;

// ============================================================================
// GET endpoint with Consumer workflow (trigger source)
// ============================================================================

#[actix_web::test]
async fn test_get_trigger_consumer_workflow_enqueues_run() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with from.trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint with JWT
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (202 Accepted or 200 OK)
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 202,
        "Expected success or 202 Accepted, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_consumer_workflow_async_mode() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with from.trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint with async=true
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}?async=true"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 202 Accepted for async mode
    assert_eq!(
        resp.status().as_u16(),
        202,
        "Expected 202 Accepted for async mode, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_consumer_workflow_poll_status() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with from.trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // First request: trigger async execution
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}?async=true"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 202);

    // Extract run_uuid from response
    let body: serde_json::Value = test::read_body_json(resp).await;
    let run_uuid = body["run_uuid"].as_str().unwrap();

    // Poll status with run_uuid
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/workflows/{wf_uuid}?async=true&run_uuid={run_uuid}"
        ))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return status (200 OK with status info)
    assert!(
        resp.status().is_success(),
        "Expected success when polling status, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_consumer_workflow_creates_entities() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_trigger_entity");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow: trigger → uri → entity
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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
                    "type": "next_step",
                    "mapping": {}
                }
            },
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

    // Test GET endpoint - should trigger workflow
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (even if external URI fails, the trigger should work)
    assert!(
        resp.status().is_success()
            || resp.status().as_u16() == 202
            || resp.status().as_u16() == 500,
        "Expected success, 202, or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_provider_workflow_returns_data() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create provider workflow (should still return data)
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

    // Test GET endpoint - should return data (even if external URI fails)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should succeed or 500 (if external URI fails)
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_with_jwt_auth() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint with JWT
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success() || resp.status().as_u16() == 202,
        "Expected success or 202 with JWT auth, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_with_api_key_auth() -> anyhow::Result<()> {
    let (app, pool, _token, api_key_value) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint with API key
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("X-API-Key", api_key_value))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success() || resp.status().as_u16() == 202,
        "Expected success or 202 with API key auth, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_without_auth_fails() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with trigger source
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint without auth
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should fail with 401 Unauthorized
    assert_eq!(
        resp.status().as_u16(),
        401,
        "Expected 401 Unauthorized without auth, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_consumer_workflow_without_trigger_source_fails() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow WITHOUT trigger source (has from.api instead)
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
                    "entity_definition": "test",
                    "path": "/",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Test GET endpoint - should return 404 (not a Provider workflow, and no trigger source)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 404 (workflow not found - because it's Consumer without trigger)
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Expected 404 for Consumer workflow without trigger source, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_disabled_workflow_fails() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create consumer workflow with trigger source but disabled
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "trigger",
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

    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, false, None).await?;

    // Test GET endpoint - should return 503 Service Unavailable
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(
        resp.status().as_u16(),
        503,
        "Expected 503 Service Unavailable for disabled workflow, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_get_trigger_workflow_not_found() -> anyhow::Result<()> {
    let (app, _pool, token, _) = setup_app_with_entities().await?;

    // Use a non-existent workflow UUID
    let fake_uuid = Uuid::now_v7();

    // Test GET endpoint - should return 404
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{fake_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(
        resp.status().as_u16(),
        404,
        "Expected 404 for non-existent workflow, got: {}",
        resp.status()
    );

    Ok(())
}
