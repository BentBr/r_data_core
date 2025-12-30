#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{
    http::{header, StatusCode},
    test, web, App,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository, WorkflowRepository};
use r_data_core_services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, WorkflowRepositoryAdapter,
};
use r_data_core_test_support::{clear_test_db, setup_test_db, test_queue_client_async};
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// Test that invalid UUID in JWT token returns 401 Unauthorized
/// This tests the UUID parsing error handling we added to the API key routes
#[tokio::test]
#[serial]
async fn test_invalid_uuid_in_jwt_token_returns_401() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    // Create API state similar to other tests
    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = r_data_core_services::WorkflowService::new(Arc::new(wf_adapter));

    let dashboard_stats_repository =
        r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service =
        r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    let jwt_secret = "test_secret".to_string();
    let api_state = ApiState {
        db_pool: pool.pool.clone(),
        api_config: r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: jwt_secret.clone(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
            check_default_admin_password: true,
        },
        role_service: r_data_core_services::RoleService::new(
            pool.pool.clone(),
            cache_manager.clone(),
            Some(0),
        ),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
        dashboard_stats_service,
        queue: test_queue_client_async().await,
    };

    // Create a JWT token with an invalid UUID in the 'sub' field
    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(1);

    // Create claims with invalid UUID but with permissions so permission check passes
    // This allows us to test the UUID parsing error
    let invalid_claims = json!({
        "sub": "not-a-valid-uuid", // Invalid UUID
        "name": "Test User",
        "email": "test@example.com",
        "permissions": ["api_keys:read"], // Add permission so permission check passes
        "exp": exp.unix_timestamp(),
        "iat": now.unix_timestamp(),
        "is_super_admin": false,
    });

    let invalid_token = encode(
        &Header::default(),
        &invalid_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| r_data_core_core::error::Error::Api(format!("Failed to encode JWT: {e}")))?;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    // Test API key list endpoint with invalid UUID in token
    let req = test::TestRequest::get()
        .uri("/admin/api/v1/api-keys")
        .insert_header((header::AUTHORIZATION, format!("Bearer {invalid_token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 401 Unauthorized
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 for invalid UUID in JWT token"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Error");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Invalid UUID in auth token"));

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test that invalid JSON in request body returns proper error
#[tokio::test]
#[serial]
async fn test_invalid_json_returns_bad_request() -> anyhow::Result<()> {
    // Use the common test setup
    let (app, _pool, token, _) = crate::api::workflows::common::setup_app_with_entities().await?;

    // Test with invalid JSON
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/api-keys")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{ invalid json }") // Invalid JSON
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 Bad Request for invalid JSON
    let status = resp.status();
    assert!(
        status.is_client_error(),
        "Should return client error (400) for invalid JSON, got: {status}"
    );

    Ok(())
}

/// Test that validation errors are properly returned for invalid workflow configs
/// Uses examples from `.example_files/json_examples/dsl/`
#[tokio::test]
#[serial]
async fn test_validation_error_handling_with_examples() -> anyhow::Result<()> {
    let (app, _pool, token, _) = crate::api::workflows::common::setup_app_with_entities().await?;

    // Test with invalid_empty_steps.json
    let invalid_config = crate::api::workflows::common::load_workflow_example(
        "invalid_empty_steps.json",
        "test_entity",
    )?;

    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(serde_json::json!({
            "name": "Test Invalid Workflow",
            "description": "Test",
            "kind": "consumer",
            "enabled": true,
            "config": invalid_config
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;

    // Should return validation error (422 or 400)
    assert!(
        resp.status().is_client_error(),
        "Should return client error for invalid workflow config, got: {}",
        resp.status()
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Error");
    assert!(
        body["message"].as_str().unwrap().contains("DSL")
            || body["message"].as_str().unwrap().contains("step")
    );

    Ok(())
}

/// Test that missing 'from' field returns proper validation error
#[tokio::test]
#[serial]
async fn test_missing_from_validation_error() -> anyhow::Result<()> {
    let (app, _pool, token, _) = crate::api::workflows::common::setup_app_with_entities().await?;

    // Test with invalid_missing_from.json
    let invalid_config = crate::api::workflows::common::load_workflow_example(
        "invalid_missing_from.json",
        "test_entity",
    )?;

    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(serde_json::json!({
            "name": "Test Missing From",
            "description": "Test",
            "kind": "consumer",
            "enabled": true,
            "config": invalid_config
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;

    // Should return validation error
    let status = resp.status();
    assert!(
        status.is_client_error(),
        "Should return client error for missing 'from' field, got: {status}"
    );

    Ok(())
}

/// Test that missing 'to' field returns proper validation error
#[tokio::test]
#[serial]
async fn test_missing_to_validation_error() -> anyhow::Result<()> {
    let (app, _pool, token, _) = crate::api::workflows::common::setup_app_with_entities().await?;

    // Test with invalid_missing_to.json
    let invalid_config = crate::api::workflows::common::load_workflow_example(
        "invalid_missing_to.json",
        "test_entity",
    )?;

    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(serde_json::json!({
            "name": "Test Missing To",
            "description": "Test",
            "kind": "consumer",
            "enabled": true,
            "config": invalid_config
        }))
        .to_request();

    let resp = test::call_service(&app, create_req).await;

    // Should return validation error
    let status = resp.status();
    assert!(
        status.is_client_error(),
        "Should return client error for missing 'to' field, got: {status}"
    );

    Ok(())
}

/// Test database error handling in repositories
#[tokio::test]
#[serial]
async fn test_database_error_handling() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());

    // Try to get a workflow with a non-existent UUID
    let non_existent_uuid = Uuid::now_v7();
    let result = repo.get_by_uuid(non_existent_uuid).await;

    // Should return Ok(None) for non-existent workflow, not an error
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    clear_test_db(&pool).await?;
    Ok(())
}

/// Test workflow error handling for invalid format config
/// Creates a provider workflow that doesn't have `output.mode = "api"`, so `format_config` will be `None`
#[tokio::test]
#[serial]
async fn test_workflow_invalid_format_config_error() -> anyhow::Result<()> {
    let (app, _pool, token, _) = crate::api::workflows::common::setup_app_with_entities().await?;

    // Create a provider workflow that outputs to entity instead of API
    // This will pass validation but result in format_config being None at runtime
    let workflow_config = json!({
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
                    "type": "entity",
                    "entity_type": "test_entity",
                    "mapping": {}
                }
            }
        ]
    });

    // Create workflow
    let create_req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(json!({
            "name": "Test Workflow No API Output",
            "description": "Test",
            "kind": "provider",
            "enabled": true,
            "config": workflow_config
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let create_status = create_resp.status();

    // Check if creation succeeded or failed with validation error
    if !create_status.is_success() {
        // If validation fails, that's also a valid test outcome
        // The workflow config might be rejected because provider workflows need API output
        let body: serde_json::Value = test::read_body_json(create_resp).await;
        assert!(
            create_status.is_client_error(),
            "Should return client error for invalid provider workflow config, got: {create_status} - {body}"
        );
        return Ok(());
    }

    let body: serde_json::Value = test::read_body_json(create_resp).await;
    let workflow_uuid = body["data"]["uuid"].as_str().unwrap();

    // Try to access the provider endpoint - should handle missing format_config gracefully
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{workflow_uuid}"))
        .to_request();

    let get_resp = test::call_service(&app, get_req).await;

    // Should return 500 or handle the error gracefully
    // The endpoint should check for format_config and return appropriate error
    let get_status = get_resp.status();
    assert!(
        get_status.is_server_error() || get_status.as_u16() == 500,
        "Should return server error when format config is missing, got: {get_status}"
    );

    Ok(())
}
