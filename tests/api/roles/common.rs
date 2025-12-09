#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)] // actix-web test utilities use Rc internally

use actix_web::{test, web, App};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, DashboardStatsRepository, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DashboardStatsService, EntityDefinitionService, RoleService,
};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, setup_test_db, test_queue_client_async,
};
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};

/// Setup a test application with all required services
///
/// # Errors
///
/// Returns an error if database setup fails or if service initialization fails.
pub async fn setup_test_app() -> Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    r_data_core_test_support::TestDatabase,
    Uuid, // user_uuid
)> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;

    let user_uuid = create_test_admin_user(&pool.pool).await?;

    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
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
    let workflow_service = WorkflowService::new(Arc::new(wf_adapter));

    let dashboard_stats_repository = DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service = DashboardStatsService::new(Arc::new(dashboard_stats_repository));

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
        },
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(3600)),
        cache_manager: cache_manager.clone(),
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
        dashboard_stats_service,
        queue: test_queue_client_async().await,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ApiStateWrapper::new(api_state)))
            .configure(configure_app),
    )
    .await;

    Ok((app, pool, user_uuid))
}

/// Create a test role with default permissions
#[must_use]
pub fn create_test_role(name: &str) -> Role {
    let mut role = Role::new(name.to_string());
    role.description = Some("Test role".to_string());

    role.permissions = vec![
        Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        },
        Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Create,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        },
    ];

    role
}
