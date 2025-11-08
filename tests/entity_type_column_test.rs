use actix_web::{test, web, App};
use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::api::{configure_app, ApiState};
use r_data_core::cache::CacheManager;
use r_data_core::config::CacheConfig;
use r_data_core::entity::admin_user::repository::{AdminUserRepository, ApiKeyRepository};
use r_data_core::entity::dynamic_entity::entity::DynamicEntity;
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::entity::entity_definition::definition::EntityDefinition;
use r_data_core::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::error::{Error, Result};
use r_data_core::services::{
    AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService, WorkflowService,
};
use r_data_core::workflow::data::repository::WorkflowRepository;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

/// Clear test database
async fn clear_test_db(pool: &PgPool) -> Result<()> {
    // Clear the main tables in reverse dependency order
    let tables = [
        "api_keys",
        "admin_users",
        "entity_definitions",
        "entities_registry",
    ];

    for table in tables.iter() {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(pool)
            .await
            .map_err(|e| Error::Database(e))?;
    }

    Ok(())
}

/// Create a test entity definition
async fn create_test_entity_definition(pool: &PgPool, entity_type: &str) -> Result<Uuid> {
    // Create a simple entity definition for testing
    let mut entity_def = EntityDefinition::default();
    entity_def.entity_type = entity_type.to_string();
    entity_def.display_name = format!("{} Class", entity_type);
    entity_def.description = Some(format!("Test description for {}", entity_type));
    entity_def.published = true;

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
    let uuid = repository.create(&entity_def).await?;

    // Trigger the view creation
    let trigger_sql = format!("SELECT create_entity_table_and_view('{}')", entity_type);
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

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
    .map_err(Error::Database)?;

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
    .map_err(Error::Database)?;

    Ok(())
}

// Helper function to hash API key or use a fallback mechanism when testing
fn use_api_key_hash_or_fallback(api_key: &str) -> String {
    use r_data_core::entity::admin_user::model::ApiKey;

    match ApiKey::hash_api_key(api_key) {
        Ok(hash) => hash,
        Err(_) => {
            // For testing purposes, use a simple hash approach if the proper method fails
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(api_key.as_bytes());
            format!("{:x}", hasher.finalize())
        }
    }
}

/// Create a test app with all required services
async fn create_test_app(
    pool: &PgPool,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    // Create a cache manager
    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    // Create repositories and services
    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
    let entity_definition_service = EntityDefinitionService::new(entity_definition_repository);

    let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        dynamic_entity_repository,
        Arc::new(entity_definition_service.clone()),
    ));

    let workflow_repository = Arc::new(WorkflowRepository::new(pool.clone()));
    let workflow_service = WorkflowService::new(workflow_repository);

    // Create app state
    let app_state = web::Data::new(ApiState {
        db_pool: pool.clone(),
        jwt_secret: "test_secret".to_string(),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(dynamic_entity_service),
        workflow_service,
    });

    // Build test app
    test::init_service(App::new().app_data(app_state).configure(configure_app)).await
}

#[actix_web::test]
async fn test_fixed_entity_type_column_issue() -> Result<()> {
    // Setup test database
    let pool = common::utils::setup_test_db().await;

    clear_test_db(&pool).await?;

    // Create a entity definition for the user entity
    let _user_def = create_test_entity_definition(&pool, "user").await?;

    // Create test users
    for i in 1..=3 {
        create_test_entity(
            &pool,
            "user",
            &format!("User {}", i),
            &format!("user{}@example.com", i),
        )
        .await?;
    }

    // Create an API key
    let api_key = "test_api_key_12345";
    create_test_api_key(&pool, api_key.to_string()).await?;

    // Build test app with DynamicEntityService, etc.
    let app = create_test_app(&pool).await;

    // Test the endpoint that previously failed due to the entity_type column
    let req = test::TestRequest::get()
        .uri("/api/v1/user")
        .insert_header(("X-API-Key", api_key))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Verify that the request now succeeds (before it would fail with DB error)
    assert!(
        resp.status().is_success(),
        "API request failed with status: {}",
        resp.status()
    );

    // Parse and verify response
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Success");
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 3);

    Ok(())
}
