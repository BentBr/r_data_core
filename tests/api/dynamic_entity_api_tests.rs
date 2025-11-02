use actix_web::{test, App, web};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use r_data_core::api::{ApiState, configure_app};
use r_data_core::cache::CacheManager;
use r_data_core::entity::class::definition::EntityDefinition;
use r_data_core::entity::dynamic_entity::entity::DynamicEntity;
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, DynamicEntityService,
};
use r_data_core::error::Result;
use std::collections::HashMap;

// Import the common module from tests
#[path = "../common/mod.rs"]
mod common;

#[cfg(test)]
mod dynamic_entity_api_tests {
    use super::*;

    // Helper to create a test entity definition for user entity
    async fn create_user_entity_definition(db_pool: &sqlx::PgPool) -> Result<Uuid> {
        // Create a simple entity definition for testing
        let mut entity_def = EntityDefinition::default();
        entity_def.entity_type = "user".to_string();
        entity_def.display_name = "Test User".to_string();
        entity_def.description = Some("Test description for User".to_string());
        entity_def.published = true;

        // Add fields to the entity definition
        let mut fields = Vec::new();

        // String field
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

        // Age field
        let age_field = FieldDefinition {
            name: "age".to_string(),
            display_name: "Age".to_string(),
            field_type: FieldType::Integer,
            required: false,
            description: Some("The age field".to_string()),
            filterable: true,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };
        fields.push(age_field);

        // Active field
        let active_field = FieldDefinition {
            name: "active".to_string(),
            display_name: "Active".to_string(),
            field_type: FieldType::Boolean,
            required: false,
            description: Some("The active field".to_string()),
            filterable: true,
            indexed: false,
            default_value: Some(json!(true)),
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };
        fields.push(active_field);

        entity_def.fields = fields;

        // Save to database
        let create_query =
            "INSERT INTO entity_definitions (uuid, entity_type, display_name, description, field_definitions, created_at, created_by, published)
             VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7) RETURNING uuid";

        let uuid = Uuid::now_v7();
        let created_by = Uuid::now_v7();

        sqlx::query(create_query)
            .bind(uuid)
            .bind(&entity_def.entity_type)
            .bind(&entity_def.display_name)
            .bind(&entity_def.description)
            .bind(json!(entity_def.fields))
            .bind(created_by)
            .bind(entity_def.published)
            .fetch_one(db_pool)
            .await
            .map_err(|e| r_data_core::error::Error::Database(e))?;

        // Execute SQL to trigger the view creation
        let trigger_sql = format!(
            "SELECT create_entity_table_and_view('{}')",
            entity_def.entity_type
        );

        sqlx::query(&trigger_sql)
            .execute(db_pool)
            .await
            .map_err(|e| r_data_core::error::Error::Database(e))?;

        // Wait a moment for the trigger to create the view
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(uuid)
    }

    // Helper to create test user entities
    async fn create_test_users(db_pool: &sqlx::PgPool, count: i32) -> Result<Vec<Uuid>> {
        let mut uuids = Vec::new();
        let repository = DynamicEntityRepository::new(db_pool.clone());

        for i in 1..=count {
            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();

            let mut field_data = HashMap::new();
            field_data.insert("uuid".to_string(), json!(uuid.to_string()));
            field_data.insert("name".to_string(), json!(format!("User {}", i)));
            field_data.insert("email".to_string(), json!(format!("user{}@example.com", i)));
            field_data.insert("age".to_string(), json!(20 + i));
            field_data.insert("active".to_string(), json!(i % 2 == 0));
            field_data.insert("created_by".to_string(), json!(created_by.to_string()));

            let entity = DynamicEntity {
                entity_type: "user".to_string(),
                field_data,
                definition: Arc::new(EntityDefinition::default()),
            };

            repository.create(&entity).await?;
            uuids.push(uuid);
        }

        Ok(uuids)
    }

    // Helper to create an API key for testing
    async fn create_test_api_key(db_pool: &sqlx::PgPool) -> Result<String> {
        // Create admin user for API key
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
        .execute(db_pool)
        .await
        .map_err(|e| r_data_core::error::Error::Database(e))?;

        // Create API key
        let api_key = "p953belra+DVlKSdUyOmOEYCOa8U4aR4lu37XD1+AwQ=".to_string();
        let api_key_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8".to_string();

        sqlx::query(
            "INSERT INTO api_keys (uuid, user_uuid, name, key_hash, is_active, created_at, created_by, published)
             VALUES ($1, $2, $3, $4, true, NOW(), $5, true)"
        )
        .bind(Uuid::now_v7())
        .bind(admin_uuid)
        .bind("test_key")
        .bind(api_key_hash)
        .bind(created_by)
        .execute(db_pool)
        .await
        .map_err(|e| r_data_core::error::Error::Database(e))?;

        Ok(api_key)
    }

    // Helper to create app test services
    async fn create_test_app(db_pool: sqlx::PgPool) -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    > {
        // Create required services
        let cache_manager = Arc::new(CacheManager::new_mock());

        let api_key_service = ApiKeyService::new(
            Arc::new(r_data_core::entity::api_key::repository::ApiKeyRepository::new(
                db_pool.clone(),
            )),
        );

        let admin_user_service = AdminUserService::new(
            Arc::new(r_data_core::entity::admin_user::AdminUserRepository::new(
                Arc::new(db_pool.clone()),
            )),
        );

        let entity_definition_service = EntityDefinitionService::new(
            Arc::new(r_data_core::entity::class::repository::EntityDefinitionRepository::new(
                db_pool.clone(),
            )),
        );

        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(db_pool.clone()));
        let dynamic_entity_service = Arc::new(DynamicEntityService::new(
            dynamic_entity_repository,
            Arc::new(entity_definition_service.clone()),
        ));

        // Create app state
        let app_state = web::Data::new(ApiState {
            db_pool: db_pool.clone(),
            jwt_secret: "test_secret".to_string(),
            cache_manager,
            api_key_service,
            admin_user_service,
            entity_definition_service,
            dynamic_entity_service: Some(dynamic_entity_service),
            workflow_service: crate::common::utils::make_workflow_service(&pool),
        });

        // Build test app
        test::init_service(
            App::new()
                .app_data(app_state)
                .configure(configure_app),
        )
        .await
    }

    #[actix_web::test]
    async fn test_get_users_api() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");

        // Create entity definition for user entity
        let _entity_uuid = create_user_entity_definition(&db_pool).await?;

        // Create test users
        let _user_uuids = create_test_users(&db_pool, 20).await?;

        // Create API key
        let api_key = create_test_api_key(&db_pool).await?;

        // Create test app
        let app = create_test_app(db_pool.clone()).await;

        // Test GET /api/v1/user endpoint with pagination
        let req = test::TestRequest::get()
            .uri("/api/v1/user?limit=5&offset=0")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        println!("Response body: {}", serde_json::to_string_pretty(&body).unwrap());

        // Check response structure
        assert_eq!(body["status"], "Success", "Response status should be Success");
        assert!(body["data"].is_array(), "Response data should be an array");
        assert_eq!(body["data"].as_array().unwrap().len(), 5, "Should return 5 users");

        // Check pagination
        assert!(body["meta"]["pagination"].is_object(), "Response should include pagination metadata");
        assert_eq!(body["meta"]["pagination"]["page"], 1, "Should be page 1");
        assert_eq!(body["meta"]["pagination"]["per_page"], 5, "Should have 5 items per page");
        assert_eq!(body["meta"]["pagination"]["total"], 20, "Should have 20 total users");
        assert_eq!(body["meta"]["pagination"]["total_pages"], 4, "Should have 4 total pages");
        assert_eq!(body["meta"]["pagination"]["has_next"], true, "Should have next page");
        assert_eq!(body["meta"]["pagination"]["has_previous"], false, "Should not have previous page");

        // Test Page 2
        let req = test::TestRequest::get()
            .uri("/api/v1/user?limit=5&offset=5")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check pagination for page 2
        assert_eq!(body["meta"]["pagination"]["page"], 2, "Should be page 2");
        assert_eq!(body["meta"]["pagination"]["has_next"], true, "Should have next page");
        assert_eq!(body["meta"]["pagination"]["has_previous"], true, "Should have previous page");

        // Test filtering - active=true
        let req = test::TestRequest::get()
            .uri("/api/v1/user?filter={\"active\":true}")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check filtered results
        let users = body["data"].as_array().unwrap();
        for user in users {
            assert_eq!(user["active"], true, "Filtered users should all be active");
        }

        // Test search functionality
        let req = test::TestRequest::get()
            .uri("/api/v1/user?q=user1")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check searched results
        let users = body["data"].as_array().unwrap();
        assert!(users.len() >= 1, "Should find at least one user matching search");

        // Check field selection
        let req = test::TestRequest::get()
            .uri("/api/v1/user?fields=name,email")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check that only requested fields are returned
        let first_user = &body["data"][0];
        assert!(first_user.get("name").is_some(), "Name field should be present");
        assert!(first_user.get("email").is_some(), "Email field should be present");
        assert!(first_user.get("age").is_none(), "Age field should not be present");

        // Test sorting functionality
        let req = test::TestRequest::get()
            .uri("/api/v1/user?sort=age&sort_direction=desc")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check that results are sorted by age in descending order
        let users = body["data"].as_array().unwrap();
        let mut last_age = i64::MAX;
        for user in users {
            let age = user["age"].as_i64().unwrap();
            assert!(age <= last_age, "Users should be sorted by age in descending order");
            last_age = age;
        }

        Ok(())
    }

    #[actix_web::test]
    async fn test_get_single_user_api() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");

        // Create entity definition for user entity
        let _entity_uuid = create_user_entity_definition(&db_pool).await?;

        // Create one test user
        let user_uuids = create_test_users(&db_pool, 1).await?;
        let user_id = user_uuids[0];

        // Create API key
        let api_key = create_test_api_key(&db_pool).await?;

        // Create test app
        let app = create_test_app(db_pool.clone()).await;

        // Test GET /api/v1/user/{id} endpoint
        let req = test::TestRequest::get()
            .uri(&format!("/api/v1/user/{}", user_id))
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "API call should succeed");

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check response structure
        assert_eq!(body["status"], "Success", "Response status should be Success");
        assert!(body["data"].is_object(), "Response data should be an object");
        assert_eq!(body["data"]["uuid"], user_id.to_string(), "UUID should match");
        assert_eq!(body["data"]["name"], "User 1", "Name should match");
        assert_eq!(body["data"]["email"], "user1@example.com", "Email should match");

        // Test with a non-existent UUID
        let fake_id = Uuid::now_v7();
        let req = test::TestRequest::get()
            .uri(&format!("/api/v1/user/{}", fake_id))
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404, "Should return 404 for non-existent entity");

        Ok(())
    }

    #[actix_web::test]
    async fn test_fixed_entity_type_column_issue() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");

        // Create entity definition for user entity
        let _entity_uuid = create_user_entity_definition(&db_pool).await?;

        // Create test users
        let _user_uuids = create_test_users(&db_pool, 5).await?;

        // Create API key
        let api_key = create_test_api_key(&db_pool).await?;

        // Create test app
        let app = create_test_app(db_pool.clone()).await;

        // Now test the API endpoint that previously would fail due to missing entity_type column
        let req = test::TestRequest::get()
            .uri("/api/v1/user")
            .insert_header(("X-API-Key", api_key.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // The request should now succeed
        assert!(
            resp.status().is_success(),
            "API call should succeed, got status: {}",
            resp.status()
        );

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check response structure
        assert_eq!(body["status"], "Success", "Response status should be Success");
        assert!(body["data"].is_array(), "Response data should be an array");

        // Test with filter parameter (previously would also fail)
        let req = test::TestRequest::get()
            .uri("/api/v1/user?filter={\"name\":\"User 1\"}")
            .insert_header(("X-API-Key", api_key))
            .to_request();

        let resp = test::call_service(&app, req).await;

        // The request should now succeed
        assert!(
            resp.status().is_success(),
            "Filtered API call should succeed, got status: {}",
            resp.status()
        );

        // Parse response body
        let body: Value = test::read_body_json(resp).await;

        // Check filtered results
        assert_eq!(body["status"], "Success", "Response status should be Success");
        let users = body["data"].as_array().unwrap();
        assert_eq!(users.len(), 1, "Should return exactly 1 filtered user");
        assert_eq!(users[0]["name"], "User 1", "Should return User 1");

        Ok(())
    }
}
