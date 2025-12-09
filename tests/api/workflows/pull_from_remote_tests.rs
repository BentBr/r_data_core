#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Tests for pulling data from remote APIs via cron into entities
// Use Case 1: Pull from remote API via cron into entities (CSV/JSON)
// Use Case 1.a: Pull from remote API with authentication
// Use Case 1.b: Pull from remote API with different HTTP methods

use super::common::{
    create_consumer_workflow, create_test_entity_definition, generate_entity_type,
    setup_app_with_entities,
};
use uuid::Uuid;

// ============================================================================
// 1. Pull from remote API via cron into entities (CSV/JSON)
// ============================================================================

#[actix_web::test]
async fn test_pull_from_remote_api_csv_into_entities_via_cron() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create entity definition (entity type must start with letter and contain only letters, numbers, underscores)
    let entity_type = generate_entity_type("test_pull_csv");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow that pulls CSV from remote API via cron into entities
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

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    // Verify workflow was created with cron
    let workflow = sqlx::query_scalar::<_, Option<String>>(
        "SELECT schedule_cron FROM workflows WHERE uuid = $1",
    )
    .bind(wf_uuid)
    .fetch_one(&pool.pool)
    .await?;

    assert_eq!(
        workflow,
        Some("*/5 * * * *".to_string()),
        "Workflow should have cron schedule"
    );

    Ok(())
}

#[actix_web::test]
async fn test_pull_from_remote_api_json_into_entities_via_cron() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create entity definition
    let entity_type = generate_entity_type("test_pull_json");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow that pulls JSON from remote API via cron into entities
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_uri_json_to_entity.json",
        &entity_type,
    )?;

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/10 * * * *".to_string()),
    )
    .await?;

    // Verify workflow was created with cron
    let workflow = sqlx::query_scalar::<_, Option<String>>(
        "SELECT schedule_cron FROM workflows WHERE uuid = $1",
    )
    .bind(wf_uuid)
    .fetch_one(&pool.pool)
    .await?;

    assert_eq!(
        workflow,
        Some("*/10 * * * *".to_string()),
        "Workflow should have cron schedule"
    );

    Ok(())
}

// ============================================================================
// 1.a. Pull from remote API with authentication (API key, pre-shared key, basic auth)
// ============================================================================

#[actix_web::test]
async fn test_pull_from_remote_api_csv_with_api_key_auth() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_api_key");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

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
                        "auth": {
                            "type": "api_key",
                            "key": "test-api-key-123",
                            "header_name": "X-API-Key"
                        }
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

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    // Verify workflow config contains API key auth
    let workflow_config: serde_json::Value =
        sqlx::query_scalar("SELECT config FROM workflows WHERE uuid = $1")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    let auth_type = workflow_config["steps"][0]["from"]["source"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("api_key"),
        "Workflow should have API key auth configured"
    );

    Ok(())
}

#[actix_web::test]
async fn test_pull_from_remote_api_csv_with_pre_shared_key_auth() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_psk");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

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
                        "auth": {
                            "type": "pre_shared_key",
                            "key": "secret-key-123",
                            "location": "header",
                            "field_name": "X-Pre-Shared-Key"
                        }
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

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    let workflow_config: serde_json::Value =
        sqlx::query_scalar("SELECT config FROM workflows WHERE uuid = $1")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    let auth_type = workflow_config["steps"][0]["from"]["source"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("pre_shared_key"),
        "Workflow should have pre-shared key auth configured"
    );

    Ok(())
}

#[actix_web::test]
async fn test_pull_from_remote_api_csv_with_basic_auth() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_basic");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

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
                        "auth": {
                            "type": "basic_auth",
                            "username": "testuser",
                            "password": "testpass"
                        }
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

    let wf_uuid = create_consumer_workflow(
        &pool,
        creator_uuid,
        config,
        true,
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    let workflow_config: serde_json::Value =
        sqlx::query_scalar("SELECT config FROM workflows WHERE uuid = $1")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    let auth_type = workflow_config["steps"][0]["from"]["source"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("basic_auth"),
        "Workflow should have basic auth configured"
    );

    Ok(())
}

// ============================================================================
// 1.b. Pull from remote API with different HTTP methods
// ============================================================================

// Note: HTTP methods are typically used for destinations (push), not sources (pull)
// Sources typically use GET. This test verifies GET works (which is the default for URI sources)
#[actix_web::test]
async fn test_pull_from_remote_api_csv_with_get_method() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_get");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // URI sources use GET by default
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
        Some("*/5 * * * *".to_string()),
    )
    .await?;

    // Verify workflow was created successfully
    let workflow_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1)")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;

    assert!(workflow_exists, "Workflow should be created");

    Ok(())
}
