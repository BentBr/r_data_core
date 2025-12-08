#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use actix_web::{test, web, App};
use r_data_core_api::{configure_app, ApiState};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::options::FieldValidation;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::admin_user_repository_trait::ApiKeyRepositoryTrait;
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
use r_data_core_services::WorkflowService;
use r_data_core_services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService,
};
use r_data_core_test_support::create_test_admin_user;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Clear test database
async fn clear_test_db(pool: &PgPool) -> Result<()> {
    // Clear the main tables in reverse dependency order
    let tables = [
        "api_keys",
        "admin_users",
        "entity_definitions",
        "entities_registry",
    ];

    for table in &tables {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;
    }

    Ok(())
}

/// Create a test entity definition
async fn create_test_entity_definition(pool: &PgPool, entity_type: &str) -> Result<Uuid> {
    // Create a simple entity definition for testing
    let mut entity_def = EntityDefinition {
        entity_type: entity_type.to_string(),
        display_name: format!("{entity_type} Class"),
        description: Some(format!("Test description for {entity_type}")),
        published: true,
        ..Default::default()
    };

    // Add fields to the entity definition
    let mut fields = Vec::new();

    // Name field
    let name_field = FieldDefinition {
        name: "name".to_string(),
        display_name: "Name".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("The name field".to_string()),
        filterable: true,
        indexed: true,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    };
    fields.push(name_field);

    // Email field
    let email_field = FieldDefinition {
        name: "email".to_string(),
        display_name: "Email".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("The email field".to_string()),
        filterable: true,
        indexed: true,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    };
    fields.push(email_field);

    entity_def.fields = fields;

    // Set created_by
    let created_by = Uuid::now_v7();
    entity_def.created_by = created_by;

    // Use the repository trait to create the entity definition
    let repository = EntityDefinitionRepository::new(pool.clone());
    let uuid = r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait::create(
        &repository,
        &entity_def,
    )
    .await?;

    // Trigger the view creation
    let trigger_sql = format!("SELECT create_entity_table_and_view('{entity_type}')");
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    // Wait a moment for the trigger
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(uuid)
}

/// Create a test entity
async fn create_test_entity(
    pool: &PgPool,
    entity_type: &str,
    name: &str,
    email: &str,
) -> Result<Uuid> {
    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("name".to_string(), json!(name));
    field_data.insert("email".to_string(), json!(email));
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert(
        "entity_key".to_string(),
        json!(format!(
            "{}-{}",
            name.to_lowercase().replace(' ', "-"),
            uuid.simple()
        )),
    );
    field_data.insert("created_by".to_string(), json!(created_by.to_string()));
    field_data.insert(
        "created_at".to_string(),
        json!(OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()),
    );
    field_data.insert("version".to_string(), json!(1));
    field_data.insert("published".to_string(), json!(true));

    let entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: Arc::new(EntityDefinition::default()),
    };

    let repository = DynamicEntityRepository::new(pool.clone());
    repository.create(&entity).await?;

    Ok(uuid)
}

/// Create a test API key
#[allow(dead_code)]
async fn create_test_api_key(pool: &PgPool, api_key: String) -> Result<()> {
    // Create an admin user first
    let admin_uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO admin_users (uuid, path, username, email, password_hash, created_at, created_by, published)
         VALUES ($1, '/users', $2, $3, $4, NOW(), $5, true)"
    )
    .bind(admin_uuid)
    .bind("test_admin")
    .bind("admin@example.com")
    .bind("$2a$12$Uei2P1KLrTSn9.XqtBBSHelmkRgJpYx2FkqKrOurOt8DG.PJiElFy") // hashed "password"
    .bind(created_by)
    .execute(pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    // Create API key
    let key_uuid = Uuid::now_v7();

    // Hash the API key using the proper method
    let key_hash = use_api_key_hash_or_fallback(&api_key);

    sqlx::query(
        "INSERT INTO api_keys (uuid, user_uuid, name, key_hash, is_active, created_at, created_by, published)
         VALUES ($1, $2, $3, $4, true, NOW(), $5, true)"
    )
    .bind(key_uuid)
    .bind(admin_uuid)
    .bind("Test API Key")
    .bind(key_hash)  // Use the hash, not the raw API key
    .bind(created_by)
    .execute(pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    Ok(())
}

// Helper function to hash API key or use a fallback mechanism when testing
#[allow(dead_code)]
fn use_api_key_hash_or_fallback(api_key: &str) -> String {
    use r_data_core_core::admin_user::ApiKey;

    ApiKey::hash_api_key(api_key).unwrap_or_else(|_| {
        // For testing purposes, use a simple hash approach if the proper method fails
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        let hash = hasher.finalize();
        format!("{hash:x}")
    })
}

/// Create a test app with all required services
#[allow(dead_code, clippy::future_not_send)] // actix-web test utilities use Rc internally
async fn create_test_app(
    pool: &PgPool,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    create_test_app_with_api_key_repo(pool, api_key_repo).await
}

#[allow(clippy::future_not_send)] // actix-web test utilities use Rc internally
async fn create_test_app_with_api_key_repo(
    pool: &PgPool,
    api_key_repo: ApiKeyRepository,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    // Create a cache manager
    let cache_config = CacheConfig {
        entity_definition_ttl: 0, // No expiration
        api_key_ttl: 600,         // 10 minutes for tests
        enabled: true,
        ttl: 3600, // 1-hour default
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    // Create repositories and services
    // Use from_repository to match the pattern in authentication_tests.rs
    // Note: We don't use cache for API keys in tests to avoid cache-related issues
    let api_key_service = ApiKeyService::from_repository(api_key_repo);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
    let entity_definition_service =
        EntityDefinitionService::new_without_cache(entity_definition_repository);

    let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        dynamic_entity_repository,
        Arc::new(entity_definition_service.clone()),
    ));

    let workflow_repository = Arc::new(WorkflowRepository::new(pool.clone()));
    let workflow_service = WorkflowService::new(workflow_repository);

    let dashboard_stats_repository =
        r_data_core_persistence::DashboardStatsRepository::new(pool.clone());
    let dashboard_stats_service =
        r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    // Create app state
    let api_state = ApiState {
        db_pool: pool.clone(),
        api_config: r_data_core_core::config::ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec![],
        },
        role_service: r_data_core_services::RoleService::new(
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
        dashboard_stats_service,
        queue: r_data_core_test_support::test_queue_client_async().await,
    };

    let app_data = web::Data::new(r_data_core_api::ApiStateWrapper::new(api_state));

    // Build test app
    test::init_service(App::new().app_data(app_data).configure(configure_app)).await
}

#[actix_web::test]
async fn test_fixed_entity_type_column_issue() -> Result<()> {
    // Setup test database
    use r_data_core_test_support::setup_test_db;
    let pool = setup_test_db().await;

    clear_test_db(&pool).await?;

    // Create a entity definition for the user entity
    let _user_def = create_test_entity_definition(&pool, "user").await?;

    // Create test users
    for i in 1..=3 {
        create_test_entity(
            &pool,
            "user",
            &format!("User {i}"),
            &format!("user{i}@example.com"),
        )
        .await?;
    }

    // Create an admin user first
    let user_uuid = create_test_admin_user(&pool).await?;

    // Create an API key repository and key BEFORE creating the app (like in authentication_tests.rs)
    let api_key_repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let (_key_uuid, api_key) = ApiKeyRepositoryTrait::create_new_api_key(
        &api_key_repo,
        "Test API Key",
        "Test Description",
        user_uuid,
        30,
    )
    .await?;

    // Build test app with DynamicEntityService, etc., using the same API key repository
    // This matches the pattern in authentication_tests.rs exactly
    let app = create_test_app_with_api_key_repo(&pool, api_key_repo).await;

    // Directly test that the API key service in the app can validate the key
    // This helps debug if there's an issue with the service setup
    let test_service =
        ApiKeyService::from_repository(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let service_validation = test_service.validate_api_key(&api_key).await?;
    assert!(
        service_validation.is_some(),
        "API key service should be able to validate the key directly"
    );

    // Test the endpoint that previously failed due to the entity_type column
    let req = test::TestRequest::get()
        .uri("/api/v1/user")
        .insert_header(("X-API-Key", api_key))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // If the request fails, print the response body for debugging
    let status = resp.status();
    if !status.is_success() {
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body);
        panic!("API request failed with status: {status}. Response body: {body_str}");
    }

    // Parse and verify response
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Success");
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 3);

    Ok(())
}
