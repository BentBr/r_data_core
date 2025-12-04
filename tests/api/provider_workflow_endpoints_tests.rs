#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use actix_web::{test, web, App};
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, ApiKeyRepositoryTrait, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
    WorkflowRepositoryAdapter,
};
use r_data_core_workflow::data::WorkflowKind;
use std::sync::Arc;
use uuid::Uuid;

// Import common test utilities
use r_data_core_test_support::{create_test_admin_user, setup_test_db, test_queue_client_async};

async fn setup_app_with_entities() -> anyhow::Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    sqlx::PgPool,
    String, // JWT token
    String, // API key value
)> {
    let pool = setup_test_db().await;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.clone()),
    ));

    // Create dynamic entity service
    let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool.clone());
    let de_adapter = r_data_core_services::adapters::DynamicEntityRepositoryAdapter::new(de_repo);
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        Arc::new(de_adapter),
        Arc::new(entity_definition_service.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = r_data_core_services::WorkflowService::new_with_entities(
        Arc::new(wf_adapter),
        dynamic_entity_service.clone(),
    );

    let jwt_secret = "test_secret".to_string();
    let api_state = ApiState {
        db_pool: pool.clone(),
        api_config: r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: jwt_secret.clone(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        },
        permission_scheme_service: r_data_core_services::PermissionSchemeService::new(
            pool.clone(),
            cache_manager.clone(),
            Some(0),
        ),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(dynamic_entity_service),
        workflow_service,
        queue: test_queue_client_async().await,
    };

    let app_data = web::Data::new(ApiStateWrapper::new(api_state));

    let app = test::init_service(
        App::new()
            .app_data(app_data.clone())
            .configure(configure_app),
    )
    .await;

    // Create test admin user and JWT
    let user_uuid = create_test_admin_user(&pool).await?;
    let user: AdminUser = sqlx::query_as("SELECT * FROM admin_users WHERE uuid = $1")
        .bind(user_uuid)
        .fetch_one(&pool)
        .await?;
    let api_config = r_data_core_core::config::ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: jwt_secret.clone(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
    };
    let token = r_data_core_api::jwt::generate_access_token(&user, &api_config, &[])?;

    // Create API key for testing - we need to use the repository directly to get the key value
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let (_api_key_uuid, api_key_value) = api_key_repo
        .create_new_api_key("test-api-key", "Test key", user_uuid, 30)
        .await?;

    Ok((app, pool, token, api_key_value))
}

async fn create_provider_workflow(
    pool: &sqlx::PgPool,
    creator_uuid: Uuid,
    config: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("provider-wf-{}", Uuid::now_v7().simple()),
        description: Some("Provider workflow test".to_string()),
        kind: WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    repo.create(&create_req, creator_uuid).await
}

async fn create_consumer_workflow_with_api_source(
    pool: &sqlx::PgPool,
    creator_uuid: Uuid,
    config: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("consumer-api-wf-{}", Uuid::now_v7().simple()),
        description: Some("Consumer workflow with API source".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    repo.create(&create_req, creator_uuid).await
}

#[actix_web::test]
async fn test_provider_endpoint_with_jwt_auth() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create provider workflow with JSON output
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

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Test GET endpoint with JWT
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should succeed (even if data fetch fails, auth should work)
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_provider_endpoint_with_api_key_auth() -> anyhow::Result<()> {
    let (app, pool, _token, api_key_value) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create provider workflow
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.json" },
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

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Test GET endpoint with API key
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("X-API-Key", api_key_value))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_provider_endpoint_with_pre_shared_key() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create provider workflow with pre-shared key auth
    let config = serde_json::json!({
        "provider_auth": {
            "type": "pre_shared_key",
            "key": "test-secret-key-123",
            "location": "header",
            "field_name": "X-Pre-Shared-Key"
        },
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.json" },
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

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Test with correct pre-shared key
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("X-Pre-Shared-Key", "test-secret-key-123"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(
        resp.status().is_success() || resp.status().as_u16() == 500,
        "Expected success or 500 with valid pre-shared key, got: {}",
        resp.status()
    );

    // Test with incorrect pre-shared key
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("X-Pre-Shared-Key", "wrong-key"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        401,
        "Expected 401 with invalid pre-shared key, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_provider_endpoint_without_auth() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;

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
                        "config": { "uri": "http://example.com/data.json" },
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

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Test without auth
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        401,
        "Expected 401 without auth, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_provider_endpoint_stats() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    let config = serde_json::json!({
        "provider_auth": {
            "type": "pre_shared_key",
            "key": "test-key",
            "location": "header",
            "field_name": "X-Key"
        },
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
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
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Test stats endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}/stats"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Stats endpoint should succeed");

    let body = test::read_body(resp).await;
    let stats: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(stats["uuid"], wf_uuid.to_string());
    assert!(stats["formats"].is_array());
    assert!(stats["auth_required"].is_boolean());

    Ok(())
}

#[actix_web::test]
async fn test_consumer_endpoint_post_with_api_source() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create consumer workflow with from.api source (accepts POST) - use format output instead of entity to avoid entity definition requirement
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

    let wf_uuid = create_consumer_workflow_with_api_source(&pool, creator_uuid, config).await?;

    // Test POST endpoint with CSV data (matching the format_type in config)
    let csv_data: Vec<u8> = b"name,email\nJohn,john@example.com".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Content-Type", "text/csv"))
        .set_payload(csv_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (202 Accepted) when workflow is enabled and has from.api source
    assert_eq!(
        resp.status().as_u16(),
        202,
        "Expected 202 Accepted, got: {}",
        resp.status()
    );

    // Verify response contains run_uuid and staged_items
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body.get("run_uuid").is_some(),
        "Response should contain run_uuid"
    );
    assert!(
        body.get("staged_items").is_some(),
        "Response should contain staged_items"
    );

    Ok(())
}

#[actix_web::test]
async fn test_consumer_endpoint_post_inactive_workflow() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create consumer workflow with from.api source but disabled
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

    // Create workflow as disabled
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("consumer-api-disabled-{}", Uuid::now_v7().simple()),
        description: Some("Consumer workflow with API source (disabled)".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: false, // Disabled
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Test POST endpoint with CSV data
    let csv_data: Vec<u8> = b"name,email\nJohn,john@example.com".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Content-Type", "text/csv"))
        .set_payload(csv_data)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 503 Service Unavailable when workflow is disabled
    assert_eq!(
        resp.status().as_u16(),
        503,
        "Expected 503 Service Unavailable for disabled workflow, got: {}",
        resp.status()
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("Workflow is not enabled"),
        "Error message should indicate workflow is not enabled"
    );

    Ok(())
}

#[actix_web::test]
async fn test_provider_endpoint_returns_404_for_consumer_workflow() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create consumer workflow (not provider)
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "test",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {}
                }
            }
        ]
    });

    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("consumer-wf-{}", Uuid::now_v7().simple()),
        description: Some("Consumer workflow".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Try to access as provider endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Expected 404 for consumer workflow, got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_consumer_endpoint_post_returns_405_for_provider_workflow() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;

    // Create provider workflow
    let config = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.json" },
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

    let wf_uuid = create_provider_workflow(&pool, creator_uuid, config).await?;

    // Try to POST to provider workflow
    let payload: Vec<u8> = b"{}".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status().as_u16(),
        405,
        "Expected 405 for provider workflow POST, got: {}",
        resp.status()
    );

    Ok(())
}
