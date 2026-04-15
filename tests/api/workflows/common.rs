#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Common test utilities for workflow E2E tests

use actix_web::{test, web, App};
use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_core::field::FieldDefinition;
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, ApiKeyRepositoryTrait, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService, LicenseService,
    WorkflowRepositoryAdapter,
};
use r_data_core_test_support::{create_test_admin_user, setup_test_db, test_queue_client_async};
use r_data_core_workflow::data::WorkflowKind;
use std::sync::Arc;
use uuid::Uuid;

// Re-export for convenience (used by workflow tests)
pub use r_data_core_test_support::create_test_entity_definition;

/// Setup test app with entities
///
/// # Errors
/// Returns an error if test setup fails
#[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
pub async fn setup_app_with_entities() -> anyhow::Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    r_data_core_test_support::TestDatabase,
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

    let license_config = LicenseConfig::default();
    let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone()),
    ));

    // Create dynamic entity service
    let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = r_data_core_services::adapters::DynamicEntityRepositoryAdapter::new(de_repo);
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        Arc::new(de_adapter),
        Arc::new(entity_definition_service.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = r_data_core_services::WorkflowService::new_with_entities(
        Arc::new(wf_adapter),
        dynamic_entity_service.clone(),
    );

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
        dynamic_entity_service: Some(dynamic_entity_service),
        workflow_service,
        dashboard_stats_service,
        queue: test_queue_client_async().await,
        license_service,
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
        .fetch_one(&pool.pool)
        .await?;
    let api_config = r_data_core_core::config::ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: jwt_secret.clone(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
        check_default_admin_password: true,
    };
    let token = r_data_core_core::admin_jwt::generate_access_token(&user, &api_config, &[])?;

    // Create API key for testing
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let (_api_key_uuid, api_key_value) = api_key_repo
        .create_new_api_key("test-api-key", "Test key", user_uuid, 30)
        .await?;

    Ok((app, pool, token, api_key_value))
}

/// Create a consumer workflow for testing
///
/// # Errors
/// Returns an error if workflow creation fails
pub async fn create_consumer_workflow(
    pool: &sqlx::PgPool,
    creator_uuid: Uuid,
    config: serde_json::Value,
    enabled: bool,
    schedule_cron: Option<String>,
) -> anyhow::Result<Uuid> {
    let repo = WorkflowRepository::new(pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("consumer-wf-{}", Uuid::now_v7().simple()),
        description: Some("Consumer workflow test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled,
        schedule_cron,
        config,
        versioning_disabled: false,
    };
    repo.create(&create_req, creator_uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Create a provider workflow for testing
///
/// # Errors
/// Returns an error if workflow creation fails
pub async fn create_provider_workflow(
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
        schedule_cron: None, // Provider workflows ignore cron
        config,
        versioning_disabled: false,
    };
    repo.create(&create_req, creator_uuid)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Generate a valid entity type name (starts with letter, contains only letters, numbers, underscores)
#[must_use]
pub fn generate_entity_type(prefix: &str) -> String {
    format!(
        "{}_{}",
        prefix,
        Uuid::now_v7().simple().to_string().replace('-', "_")
    )
}

/// Load a workflow DSL example from `.example_files` and substitute `entity_type`
///
/// # Arguments
/// * `filename` - The filename in `.example_files/json_examples/dsl/`
/// * `entity_type` - The entity type to substitute for `${ENTITY_TYPE}` placeholder
///
/// # Errors
/// Returns an error if the file cannot be read or parsed
pub fn load_workflow_example(
    filename: &str,
    entity_type: &str,
) -> anyhow::Result<serde_json::Value> {
    use std::fs;
    let path = format!(".example_files/json_examples/dsl/{filename}");
    let content =
        fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;
    let content = content.replace("${ENTITY_TYPE}", entity_type);
    serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse {path}: {e}"))
}

/// Create an entity definition with custom fields
///
/// # Errors
/// Returns an error if entity definition creation fails
pub async fn create_entity_definition_with_fields(
    pool: &sqlx::PgPool,
    entity_type: &str,
    fields: Vec<FieldDefinition>,
) -> anyhow::Result<Uuid> {
    let mut entity_def = r_data_core_core::entity_definition::EntityDefinition {
        entity_type: entity_type.to_string(),
        display_name: format!("{entity_type} Class"),
        description: Some(format!("Test description for {entity_type}")),
        published: true,
        fields,
        ..r_data_core_core::entity_definition::EntityDefinition::default()
    };

    let created_by = Uuid::now_v7();
    entity_def.created_by = created_by;

    let repository = r_data_core_persistence::EntityDefinitionRepository::new(pool.clone());
    let service = EntityDefinitionService::new_without_cache(Arc::new(repository));

    let uuid = service.create_entity_definition(&entity_def).await?;

    // Wait a moment for the view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(uuid)
}
