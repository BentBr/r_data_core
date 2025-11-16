// Common test utilities for workflow E2E tests

use actix_web::{test, web, App};
use r_data_core::api::{configure_app, ApiState};
use r_data_core::cache::CacheManager;
use r_data_core::config::CacheConfig;
use r_data_core::entity::admin_user::model::AdminUser;
use r_data_core::entity::admin_user::repository::{AdminUserRepository, ApiKeyRepository};
use r_data_core::entity::admin_user::repository_trait::ApiKeyRepositoryTrait;
use r_data_core::services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
    WorkflowRepositoryAdapter,
};
use r_data_core::workflow::data::repository::WorkflowRepository;
use r_data_core::workflow::data::WorkflowKind;
use std::sync::Arc;
use uuid::Uuid;

// Import common test utilities
#[path = "../../common/mod.rs"]
mod common;

pub async fn setup_app_with_entities() -> anyhow::Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    sqlx::PgPool,
    String, // JWT token
    String, // API key value
)> {
    let pool = common::utils::setup_test_db().await;

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
        r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository::new(
            pool.clone(),
        ),
    ));

    // Create dynamic entity service
    let de_repo =
        r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository::new(pool.clone());
    let de_adapter = r_data_core::services::DynamicEntityRepositoryAdapter::new(de_repo);
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        Arc::new(de_adapter),
        Arc::new(entity_definition_service.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service =
        r_data_core::services::workflow_service::WorkflowService::new_with_entities(
            Arc::new(wf_adapter),
            dynamic_entity_service.clone(),
        );

    let jwt_secret = "test_secret".to_string();
    let app_state = web::Data::new(ApiState {
        db_pool: pool.clone(),
        jwt_secret: jwt_secret.clone(),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(dynamic_entity_service),
        workflow_service,
        queue: common::utils::test_queue_client_async().await,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .configure(configure_app),
    )
    .await;

    // Create test admin user and JWT
    let user_uuid = common::utils::create_test_admin_user(&pool).await?;
    let user: AdminUser = sqlx::query_as("SELECT * FROM admin_users WHERE uuid = $1")
        .bind(user_uuid)
        .fetch_one(&pool)
        .await?;
    let token = r_data_core::api::jwt::generate_access_token(&user, &jwt_secret)?;

    // Create API key for testing
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let (_api_key_uuid, api_key_value) = api_key_repo
        .create_new_api_key("test-api-key", "Test key", user_uuid, 30)
        .await?;

    Ok((app, pool, token, api_key_value))
}

pub async fn create_test_entity_definition(
    pool: &sqlx::PgPool,
    entity_type: &str,
) -> anyhow::Result<Uuid> {
    common::utils::create_test_entity_definition(pool, entity_type)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create entity definition: {}", e))
}

pub async fn create_consumer_workflow(
    pool: &sqlx::PgPool,
    creator_uuid: Uuid,
    config: serde_json::Value,
    enabled: bool,
    schedule_cron: Option<String>,
) -> anyhow::Result<Uuid> {
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("consumer-wf-{}", Uuid::now_v7()),
        description: Some("Consumer workflow test".to_string()),
        kind: WorkflowKind::Consumer,
        enabled,
        schedule_cron,
        config,
        versioning_disabled: false,
    };
    Ok(repo.create(&create_req, creator_uuid).await?)
}

pub async fn create_provider_workflow(
    pool: &sqlx::PgPool,
    creator_uuid: Uuid,
    config: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("provider-wf-{}", Uuid::now_v7()),
        description: Some("Provider workflow test".to_string()),
        kind: WorkflowKind::Provider,
        enabled: true,
        schedule_cron: None, // Provider workflows ignore cron
        config,
        versioning_disabled: false,
    };
    Ok(repo.create(&create_req, creator_uuid).await?)
}

/// Generate a valid entity type name (starts with letter, contains only letters, numbers, underscores)
pub fn generate_entity_type(prefix: &str) -> String {
    format!(
        "{}_{}",
        prefix,
        Uuid::now_v7().simple().to_string().replace('-', "_")
    )
}
