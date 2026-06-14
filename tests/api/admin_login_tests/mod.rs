#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

//! Integration tests for admin login security hardening, split by concern:
//!   - `lockout`     — account lockout after 5 failures + reset on success
//!   - `rate_limit`  — per-IP request throttling (429) and its reset on success
//!
//! Shared `setup_app` / login helpers live here so both submodules reuse one
//! test-app factory.

mod lockout;
mod rate_limit;

use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test, web, App};
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, DashboardStatsRepository, EntityDefinitionRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService,
    LicenseService, RoleService,
};
use r_data_core_test_support::{
    clear_test_db, make_workflow_service, setup_test_db, test_queue_client_async, TestDatabase,
};
use std::net::SocketAddr;
use std::sync::Arc;

/// Build a fresh test app with an enabled in-process cache (so the per-IP
/// rate-limit counter persists across requests within one test).
pub(super) async fn setup_app() -> r_data_core_core::error::Result<(
    impl Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
    TestDatabase,
)> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 1000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let entity_def_repo = Arc::new(EntityDefinitionRepository::new(pool.pool.clone()));

    let license_service = Arc::new(LicenseService::new(
        LicenseConfig::default(),
        cache_manager.clone(),
    ));

    let api_state = ApiState {
        db_pool: pool.pool.clone(),
        api_config: r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
            check_default_admin_password: false,
        },
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(0)),
        cache_manager: cache_manager.clone(),
        api_key_service: ApiKeyService::from_repository(api_key_repo),
        admin_user_service: AdminUserService::from_repository(admin_user_repo),
        entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
        dynamic_entity_service: None,
        workflow_service: make_workflow_service(&pool),
        dashboard_stats_service: DashboardStatsService::new(Arc::new(
            DashboardStatsRepository::new(pool.pool.clone()),
        )),
        queue: test_queue_client_async().await,
        license_service,
        password_reset_service: None,
        system_log_service: None,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    Ok((app, pool))
}

/// Attempt a login (no explicit client IP) and return the HTTP status.
pub(super) async fn attempt_login<S>(app: &S, username: &str, password: &str) -> StatusCode
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({ "username": username, "password": password }))
        .to_request();
    test::call_service(app, req).await.status()
}

/// Attempt a login from a specific client IP so the per-IP rate-limit key is
/// deterministic (`req.peer_addr()` is otherwise unset in the test harness).
pub(super) async fn attempt_login_from_ip<S>(
    app: &S,
    ip: SocketAddr,
    username: &str,
    password: &str,
) -> StatusCode
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .peer_addr(ip)
        .set_json(serde_json::json!({ "username": username, "password": password }))
        .to_request();
    test::call_service(app, req).await.status()
}
