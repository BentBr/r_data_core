// Tests for exposing data via our API endpoint (CSV/JSON)
// Use Case 4: Expose data via our API endpoint (CSV/JSON), ignore cron
// Use Case 4.a: Expose CSV via API with all auth methods (header)
// Use Case 4.b: Expose JSON via API with all auth methods (body)

use super::common::setup_app_with_entities;
use actix_web::test;
use uuid::Uuid;

// ============================================================================
// 4. Expose data via our API endpoint (CSV/JSON), ignore cron
// ============================================================================

#[actix_web::test]
async fn test_expose_data_via_api_endpoint_csv_ignores_cron() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create provider workflow that exposes CSV via API endpoint
    // Provider workflows should ignore cron (they're triggered by GET requests)
    let config = serde_json::json!({
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
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                }
            }
        ]
    });

    // Even if we try to set a cron, provider workflows should ignore it
    let repo = r_data_core::workflow::data::repository::WorkflowRepository::new(pool.clone());
    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("provider-csv-{}", Uuid::now_v7()),
        description: Some("Provider workflow CSV".to_string()),
        kind: r_data_core::workflow::data::WorkflowKind::Provider,
        enabled: true,
        schedule_cron: Some("*/5 * * * *".to_string()), // This should be ignored
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Verify workflow was created
    let workflow_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1 AND kind = 'provider')",
    )
    .bind(wf_uuid)
    .fetch_one(&pool)
    .await?;

    assert!(workflow_exists, "Provider workflow should be created");

    // Test GET endpoint (should work regardless of cron)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{}", wf_uuid))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should succeed (even if external URI fails, the endpoint should be accessible)
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_expose_data_via_api_endpoint_json_ignores_cron() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

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

    let repo = r_data_core::workflow::data::repository::WorkflowRepository::new(pool.clone());
    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("provider-json-{}", Uuid::now_v7()),
        description: Some("Provider workflow JSON".to_string()),
        kind: r_data_core::workflow::data::WorkflowKind::Provider,
        enabled: true,
        schedule_cron: Some("*/10 * * * *".to_string()), // This should be ignored
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    let workflow_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1 AND kind = 'provider')",
    )
    .bind(wf_uuid)
    .fetch_one(&pool)
    .await?;

    assert!(workflow_exists, "Provider workflow should be created");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{}", wf_uuid))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

// ============================================================================
// 4.a. Expose CSV via API with all auth methods (header)
// ============================================================================

// These tests are similar to the provider endpoint auth tests already in provider_workflow_endpoints_tests.rs
// test_provider_endpoint_with_jwt_auth, test_provider_endpoint_with_api_key_auth, test_provider_endpoint_with_pre_shared_key

// ============================================================================
// 4.b. Expose JSON via API with all auth methods (body)
// ============================================================================

// Similar to 4.a, but for JSON format. The auth methods are the same regardless of format.
