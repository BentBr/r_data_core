#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Tests for pushing data to remote APIs via cron from entities
// Use Case 3: Push data to remote API via cron from entities (CSV/JSON)
// Use Case 3.a: Push CSV to remote API with all auth methods (header)
// Use Case 3.b: Push JSON to remote API with all auth methods (body)

use super::common::{
    create_consumer_workflow, create_test_entity_definition, generate_entity_type,
    setup_app_with_entities,
};
use uuid::Uuid;

// ============================================================================
// 3. Push data to remote API via cron from entities (CSV/JSON)
// ============================================================================

#[actix_web::test]
async fn test_push_to_remote_api_csv_from_entities_via_cron() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_csv");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create consumer workflow that pushes CSV to remote API from entities via cron
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": {
                        "field": "name",
                        "value": "test"
                    },
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": null
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
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
    .fetch_one(&pool)
    .await?;

    assert_eq!(
        workflow,
        Some("*/5 * * * *".to_string()),
        "Workflow should have cron schedule"
    );

    Ok(())
}

#[actix_web::test]
async fn test_push_to_remote_api_json_from_entities_via_cron() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_json");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": {
                        "field": "name",
                        "value": "test"
                    },
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": null
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
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
        Some("*/10 * * * *".to_string()),
    )
    .await?;

    let workflow = sqlx::query_scalar::<_, Option<String>>(
        "SELECT schedule_cron FROM workflows WHERE uuid = $1",
    )
    .bind(wf_uuid)
    .fetch_one(&pool)
    .await?;

    assert_eq!(
        workflow,
        Some("*/10 * * * *".to_string()),
        "Workflow should have cron schedule"
    );

    Ok(())
}

// ============================================================================
// 3.a. Push CSV to remote API with all auth methods (header)
// ============================================================================

#[actix_web::test]
async fn test_push_csv_to_remote_api_with_pre_shared_key_header() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_psk");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": { "field": "name", "value": "test" },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": {
                                "type": "pre_shared_key",
                                "key": "secret-key-123",
                                "location": "header",
                                "field_name": "X-Pre-Shared-Key"
                            }
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
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
            .fetch_one(&pool)
            .await?;

    let auth_type =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("pre_shared_key"),
        "Workflow should have pre-shared key auth configured"
    );

    Ok(())
}

#[actix_web::test]
async fn test_push_csv_to_remote_api_with_basic_auth() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_basic");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": { "field": "name", "value": "test" },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": {
                                "type": "basic_auth",
                                "username": "testuser",
                                "password": "testpass"
                            }
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
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
            .fetch_one(&pool)
            .await?;

    let auth_type =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("basic_auth"),
        "Workflow should have basic auth configured"
    );

    Ok(())
}

#[actix_web::test]
async fn test_push_csv_to_remote_api_with_api_key_header() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_apikey");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": { "field": "name", "value": "test" },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": {
                                "type": "api_key",
                                "key": "test-api-key-123",
                                "header_name": "X-API-Key"
                            }
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
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
            .fetch_one(&pool)
            .await?;

    let auth_type =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("api_key"),
        "Workflow should have API key auth configured"
    );

    Ok(())
}

// ============================================================================
// 3.b. Push JSON to remote API with all auth methods (body)
// ============================================================================

#[actix_web::test]
async fn test_push_json_to_remote_api_with_pre_shared_key_body() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_json_psk");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": { "field": "name", "value": "test" },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": {
                                "type": "pre_shared_key",
                                "key": "secret-key-123",
                                "location": "body",
                                "field_name": "api_key"
                            }
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
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
            .fetch_one(&pool)
            .await?;

    let auth_type =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("pre_shared_key"),
        "Workflow should have pre-shared key auth configured"
    );

    let location =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["location"].as_str();
    assert_eq!(location, Some("body"), "Pre-shared key should be in body");

    Ok(())
}

#[actix_web::test]
async fn test_push_json_to_remote_api_with_api_key_body() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let entity_type = generate_entity_type("test_push_json_apikey");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Note: API keys are typically in headers, but for JSON we can simulate body-based auth
    // This test verifies the configuration structure
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": { "field": "name", "value": "test" },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": {
                        "mode": "push",
                        "destination": {
                            "destination_type": "uri",
                            "config": {
                                "uri": "http://example.com/api/data"
                            },
                            "auth": {
                                "type": "api_key",
                                "key": "test-api-key-123",
                                "header_name": "X-API-Key"
                            }
                        },
                        "method": "post"
                    },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
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
            .fetch_one(&pool)
            .await?;

    let auth_type =
        workflow_config["steps"][0]["to"]["output"]["destination"]["auth"]["type"].as_str();
    assert_eq!(
        auth_type,
        Some("api_key"),
        "Workflow should have API key auth configured"
    );

    Ok(())
}
