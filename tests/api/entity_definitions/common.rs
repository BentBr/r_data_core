#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Common test utilities for entity definition constraint tests.

use actix_web::{test, web, App};
use r_data_core_api::ApiState;
use r_data_core_core::admin_jwt::AuthUserClaims;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository, EntityDefinitionRepository};
use r_data_core_services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, LicenseService,
};
use r_data_core_test_support::{test_queue_client_async, TestDatabase};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// Creates a test JWT token for authentication.
///
/// # Panics
///
/// Panics if JWT encoding fails, which should not happen in tests.
pub fn create_test_jwt_token(user_uuid: &Uuid, secret: &str) -> String {
    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(1);

    let permissions = [
        "entity_definitions:read",
        "entity_definitions:create",
        "entity_definitions:update",
        "entity_definitions:delete",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();

    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        iss: r_data_core_core::admin_jwt::ADMIN_JWT_ISSUER.to_string(),
        name: "test_user".to_string(),
        email: "test@example.com".to_string(),
        permissions,
        exp: usize::try_from(exp.unix_timestamp()).unwrap_or(0),
        iat: usize::try_from(now.unix_timestamp()).unwrap_or(0),
        is_super_admin: false,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create JWT token")
}

/// Creates a test Actix app with all required services configured.
#[allow(clippy::future_not_send)]
pub async fn create_test_app(
    pool: &TestDatabase,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let entity_def_repo = Arc::new(EntityDefinitionRepository::new(pool.pool.clone()));
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let admin_user_repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

    let cache_config = CacheConfig {
        entity_definition_ttl: 0, // Disable cache for tests
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 1000,
    };

    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let dashboard_stats_repository =
        r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service =
        r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    let license_config = LicenseConfig::default();
    let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

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
            check_default_admin_password: true,
        },
        role_service: r_data_core_services::RoleService::new(
            pool.pool.clone(),
            cache_manager.clone(),
            Some(0),
        ),
        cache_manager: cache_manager.clone(),
        api_key_service: ApiKeyService::from_repository(api_key_repo),
        admin_user_service: AdminUserService::from_repository(admin_user_repo),
        entity_definition_service: EntityDefinitionService::new_without_cache(entity_def_repo),
        dynamic_entity_service: None,
        workflow_service: r_data_core_services::WorkflowService::new(Arc::new(
            r_data_core_services::WorkflowRepositoryAdapter::new(
                r_data_core_persistence::WorkflowRepository::new(pool.pool.clone()),
            ),
        )),
        dashboard_stats_service,
        queue: test_queue_client_async().await,
        license_service,
    };

    test::init_service(
        App::new()
            .app_data(web::Data::new(r_data_core_api::ApiStateWrapper::new(
                api_state,
            )))
            .service(
                web::scope("/admin/api/v1").service(web::scope("/entity-definitions").configure(
                    r_data_core_api::admin::entity_definitions::routes::register_routes,
                )),
            ),
    )
    .await
}
