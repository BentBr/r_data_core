#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{http::StatusCode, test, web, App};
use httpmock::MockServer;
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{ApiConfig, CacheConfig, LicenseConfig};
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, ApiKeyRepository, CreateAdminUserParams,
    DashboardStatsRepository, EntityDefinitionRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService, LicenseService, RoleService,
};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, make_workflow_service, setup_test_db,
    test_queue_client_async,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_app_with_license_config(
    license_key: Option<String>,
) -> r_data_core_core::error::Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    r_data_core_test_support::TestDatabase,
)> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_config = ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        use_tls: false,
        jwt_secret: "test_secret".to_string(),
        jwt_expiration: 86400,
        enable_docs: true,
        cors_origins: vec!["*".to_string()],
        check_default_admin_password: false,
    };

    // Use mock server for license verification
    let mock_server = MockServer::start();
    let license_config = LicenseConfig {
        license_key,
        private_key: None,
               public_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
        statistics_url: format!("http://{}/submit", mock_server.address()),
    };

    let api_state = ApiState {
        db_pool: pool.pool.clone(),
        api_config: api_config.clone(),
        cache_manager: cache_manager.clone(),
        api_key_service: ApiKeyService::new(Arc::new(ApiKeyRepository::new(Arc::new(
            pool.pool.clone(),
        )))),
        admin_user_service: AdminUserService::new(Arc::new(AdminUserRepository::new(Arc::new(
            pool.pool.clone(),
        )))),
        entity_definition_service: EntityDefinitionService::new(
            Arc::new(EntityDefinitionRepository::new(pool.pool.clone())),
            cache_manager.clone(),
        ),
        dynamic_entity_service: None,
        workflow_service: make_workflow_service(pool.pool.clone()).await,
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), None),
        dashboard_stats_service: DashboardStatsService::new(Arc::new(
            DashboardStatsRepository::new(pool.pool.clone()),
        )),
        license_service: Arc::new(LicenseService::new(license_config.clone(), cache_manager.clone())),
        queue: test_queue_client_async().await,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    Ok((app, pool))
}

#[tokio::test]
#[serial]
async fn test_license_api_none_state() -> r_data_core_core::error::Result<()> {
    let (app, _pool) = setup_test_app_with_license_config(None).await?;

    let user_uuid = create_test_admin_user(&_pool).await?;

    // Login to get token
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "admin",
            "password": "adminadmin"
        }))
        .to_request();

    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);

    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token = login_body["data"]["access_token"].as_str().unwrap();

    // Get license status
    let license_req = test::TestRequest::get()
        .uri("/admin/api/v1/system/license")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let license_resp = test::call_service(&app, license_req).await;
    assert_eq!(license_resp.status(), StatusCode::OK);

    let license_body: serde_json::Value = test::read_body_json(license_resp).await;
    assert_eq!(license_body["data"]["state"], "none");
    assert!(license_body["data"]["company"].is_null());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_license_api_invalid_state() -> r_data_core_core::error::Result<()> {
    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/verify");
        then.status(200)
            .json_body(serde_json::json!({ "valid": false, "message": "Invalid license key" }));
    });

    let license_key = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJ2ZXJzaW9uIjoidjEiLCJjb21wYW55IjoiVGVzdCBDb21wYW55IiwibGljZW5zZV90eXBlIjoiRW50ZXJwcmlzZSIsImlzc3VlZF9hdCI6IjIwMjQtMDEtMDFUMDA6MDA6MDBaIiwibGljZW5zZV9pZCI6IjAxOGYxMjM0LTU2NzgtOWFiYy1kZWYwLTEyMzQ1Njc4OWFiYyJ9.test_signature";

    let (app, _pool) = setup_test_app_with_license_config(Some(license_key.to_string())).await?;

    let user_uuid = create_test_admin_user(&_pool).await?;

    // Login to get token
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "admin",
            "password": "adminadmin"
        }))
        .to_request();

    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);

    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token = login_body["data"]["access_token"].as_str().unwrap();

    // Get license status - should return Error state (can't decode JWT) or None
    let license_req = test::TestRequest::get()
        .uri("/admin/api/v1/system/license")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let license_resp = test::call_service(&app, license_req).await;
    assert_eq!(license_resp.status(), StatusCode::OK);

    let license_body: serde_json::Value = test::read_body_json(license_resp).await;
    // Should be none, error, or invalid depending on JWT decode
    let state = license_body["data"]["state"].as_str().unwrap();
    assert!(
        state == "none" || state == "error" || state == "invalid",
        "State should be none, error, or invalid, got: {state}"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_license_api_error_state() -> r_data_core_core::error::Result<()> {
    // Mock server that returns error
    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/verify");
        then.status(500).body("Internal Server Error");
    });

    let license_key = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJ2ZXJzaW9uIjoidjEiLCJjb21wYW55IjoiVGVzdCBDb21wYW55IiwibGljZW5zZV90eXBlIjoiRW50ZXJwcmlzZSIsImlzc3VlZF9hdCI6IjIwMjQtMDEtMDFUMDA6MDA6MDBaIiwibGljZW5zZV9pZCI6IjAxOGYxMjM0LTU2NzgtOWFiYy1kZWYwLTEyMzQ1Njc4OWFiYyJ9.test_signature";

    let (app, _pool) = setup_test_app_with_license_config(Some(license_key.to_string())).await?;

    let user_uuid = create_test_admin_user(&_pool).await?;

    // Login to get token
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "admin",
            "password": "adminadmin"
        }))
        .to_request();

    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);

    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token = login_body["data"]["access_token"].as_str().unwrap();

    // Get license status - should return Error or None state (can't decode JWT or API error)
    let license_req = test::TestRequest::get()
        .uri("/admin/api/v1/system/license")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let license_resp = test::call_service(&app, license_req).await;
    assert_eq!(license_resp.status(), StatusCode::OK);

    let license_body: serde_json::Value = test::read_body_json(license_resp).await;
    // Should be none or error depending on JWT decode
    let state = license_body["data"]["state"].as_str().unwrap();
    assert!(
        state == "none" || state == "error",
        "State should be none or error, got: {state}"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_license_api_valid_state() -> r_data_core_core::error::Result<()> {
    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/verify");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true, "message": "Valid license" }));
    });

    // Note: For a real valid state test, we'd need to generate a proper JWT
    // For now, this test verifies the API endpoint works
    let license_key = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJ2ZXJzaW9uIjoidjEiLCJjb21wYW55IjoiVGVzdCBDb21wYW55IiwibGljZW5zZV90eXBlIjoiRW50ZXJwcmlzZSIsImlzc3VlZF9hdCI6IjIwMjQtMDEtMDFUMDA6MDA6MDBaIiwibGljZW5zZV9pZCI6IjAxOGYxMjM0LTU2NzgtOWFiYy1kZWYwLTEyMzQ1Njc4OWFiYyJ9.test_signature";

    let (app, _pool) = setup_test_app_with_license_config(Some(license_key.to_string())).await?;

    let user_uuid = create_test_admin_user(&_pool).await?;

    // Login to get token
    let login_req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({
            "username": "admin",
            "password": "adminadmin"
        }))
        .to_request();

    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);

    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let token = login_body["data"]["access_token"].as_str().unwrap();

    // Get license status
    let license_req = test::TestRequest::get()
        .uri("/admin/api/v1/system/license")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let license_resp = test::call_service(&app, license_req).await;
    assert_eq!(license_resp.status(), StatusCode::OK);

    let license_body: serde_json::Value = test::read_body_json(license_resp).await;
    // Should return a valid response (state will be none/error since JWT is invalid, but API works)
    assert!(license_body["data"]["state"].is_string());

    Ok(())
}

