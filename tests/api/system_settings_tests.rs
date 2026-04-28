#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{http::StatusCode, test};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{ApiConfig, CacheConfig, LicenseConfig};
use r_data_core_core::error::Result;
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, DashboardStatsRepository, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService,
    LicenseService, RoleService, WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core_test_support::clear_test_db;
use serial_test::serial;
use std::sync::Arc;

use crate::api::users::common::get_auth_token;
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};

#[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
async fn maybe_setup_test_app() -> Option<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    r_data_core_test_support::TestDatabase,
)> {
    let Some(pool) = r_data_core_test_support::try_setup_test_db().await else {
        eprintln!("Skipping API system settings test: test database not available");
        return None;
    };
    if let Err(e) = clear_test_db(&pool.pool).await {
        eprintln!("Skipping API system settings test: failed to clear test database: {e}");
        return None;
    }

    if r_data_core_test_support::create_test_admin_user(&pool)
        .await
        .is_err()
    {
        eprintln!("Skipping API system settings test: failed to create admin user");
        return None;
    }

    let cache_manager = Arc::new(CacheManager::new(CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    }));

    let license_service = Arc::new(LicenseService::new(
        LicenseConfig::default(),
        cache_manager.clone(),
    ));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);
    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);
    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone()),
    ));
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = WorkflowService::new(Arc::new(wf_adapter));
    let dashboard_stats_repository = DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service = DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    let api_state = ApiState {
        db_pool: pool.pool.clone(),
        api_config: ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
            check_default_admin_password: true,
        },
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600)),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
        dashboard_stats_service,
        queue: r_data_core_test_support::test_queue_client_async().await,
        license_service,
        password_reset_service: None,
        system_log_service: None,
    };

    let app = test::init_service(
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    Some((app, pool))
}

#[tokio::test]
#[serial]
async fn get_outbox_settings_requires_authentication() -> Result<()> {
    let Some((app, pool)) = maybe_setup_test_app().await else {
        return Ok(());
    };

    let req = test::TestRequest::get()
        .uri("/admin/api/v1/system/settings/outbox")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    clear_test_db(&pool.pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn get_outbox_settings_returns_defaults() -> Result<()> {
    let Some((app, pool)) = maybe_setup_test_app().await else {
        return Ok(());
    };
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri("/admin/api/v1/system/settings/outbox")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["fetch_enabled"], false);
    assert_eq!(body["data"]["push_enabled"], true);

    clear_test_db(&pool.pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn update_outbox_settings_persists_and_is_readable() -> Result<()> {
    let Some((app, pool)) = maybe_setup_test_app().await else {
        return Ok(());
    };
    let token = get_auth_token(&app, &pool).await;

    let update_req = test::TestRequest::put()
        .uri("/admin/api/v1/system/settings/outbox")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "fetch_enabled": true,
            "push_enabled": false
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);
    let update_body: serde_json::Value = test::read_body_json(update_resp).await;
    assert_eq!(update_body["data"]["fetch_enabled"], true);
    assert_eq!(update_body["data"]["push_enabled"], false);

    let get_req = test::TestRequest::get()
        .uri("/admin/api/v1/system/settings/outbox")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(get_body["data"]["fetch_enabled"], true);
    assert_eq!(get_body["data"]["push_enabled"], false);

    clear_test_db(&pool.pool).await?;
    Ok(())
}
