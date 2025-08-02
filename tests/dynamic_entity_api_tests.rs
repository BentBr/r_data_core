use actix_web::test;
use log::warn;
use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::entity::dynamic_entity::entity::DynamicEntity;
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use r_data_core::entity::entity_definition::definition::EntityDefinition;
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::error::{Error, Result};
use r_data_core::services::{DynamicEntityService, EntityDefinitionService};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Once;
use time::OffsetDateTime;
use uuid::Uuid;

// Import the common module from tests
#[path = "common/mod.rs"]
mod common;

// Force tests to run sequentially to avoid database contention
#[cfg(test)]
mod dynamic_entity_tests {
    use super::*;

    // Initialize test framework once
    static INIT: Once = Once::new();

    fn test_setup() {
        INIT.call_once(|| {
            // Initialize logging for better test output
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .init();
        });
    }

    // Helper function to generate a unique entity type name
    fn unique_entity_type(base: &str) -> String {
        common::utils::unique_entity_type(base)
    }

    // Helper to create a test entity definition
    async fn create_test_entity_definition(
        db_pool: &PgPool,
        entity_type: &str,
    ) -> Result<(Uuid, EntityDefinition)> {
        // Create a simple entity definition for testing
        let mut entity_def = EntityDefinition::new(
            entity_type.to_string(),
            format!("Test {}", entity_type),
            Some(format!("Test {} description", entity_type)),
            None,
            false,
            None,
            Vec::new(),
            Uuid::now_v7(),
        );

        entity_def.published = true; // Ensure the class is published

        // Add some fields
        let name_field = FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("User's full name".to_string()),
            filterable: true,
            indexed: true,
            default_value: None,
            validation: FieldValidation {
                min_length: Some(2),
                max_length: Some(100),
                ..Default::default()
            },
            ui_settings: UiSettings {
                width: Some(12),
                help_text: Some("User's full name".to_string()),
                placeholder: Some("Enter full name".to_string()),
                ..Default::default()
            },
            constraints: HashMap::new(),
        };

        let email_field = FieldDefinition {
            name: "email".to_string(),
            display_name: "Email Address".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("Email address".to_string()),
            filterable: true,
            indexed: true,
            default_value: None,
            validation: FieldValidation {
                pattern: Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string()),
                max_length: Some(255),
                ..Default::default()
            },
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let age_field = FieldDefinition {
            name: "age".to_string(),
            display_name: "Age".to_string(),
            field_type: FieldType::Integer,
            required: false,
            description: Some("User's age".to_string()),
            filterable: true,
            indexed: false,
            default_value: None,
            validation: FieldValidation {
                min_value: Some(json!(0)),
                max_value: Some(json!(120)),
                ..Default::default()
            },
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let active_field = FieldDefinition {
            name: "active".to_string(),
            display_name: "Active Status".to_string(),
            field_type: FieldType::Boolean,
            required: false,
            description: Some("Whether the user is active".to_string()),
            filterable: true,
            indexed: false,
            default_value: Some(json!(true)),
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        // Add fields to the entity definition
        entity_def.fields = vec![name_field, email_field, age_field, active_field];

        // Save to a database using the service
        let entity_definition_repository = EntityDefinitionRepository::new(db_pool.clone());
        let entity_definition_service =
            EntityDefinitionService::new(Arc::new(entity_definition_repository));
        let uuid = entity_definition_service
            .create_entity_definition(&entity_def)
            .await?;

        // Wait a moment for the service to create the necessary database objects
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify the entity definition was created successfully
        let created_def = entity_definition_service
            .get_entity_definition(&uuid)
            .await?;
        assert_eq!(
            created_def.entity_type, entity_type,
            "Class definition entity type mismatch"
        );

        Ok((uuid, entity_def))
    }

    // Helper to create a test dynamic entity with the associated entity definition
    fn create_test_entity(
        entity_type: &str,
        entity_def: Arc<EntityDefinition>,
        uuid: Option<Uuid>,
    ) -> DynamicEntity {
        let mut entity = DynamicEntity {
            entity_type: entity_type.to_string(),
            field_data: HashMap::new(),
            definition: entity_def,
        };

        // Add field data
        let entity_uuid = uuid.unwrap_or_else(Uuid::now_v7);
        let created_by = Uuid::now_v7();

        entity
            .field_data
            .insert("uuid".to_string(), json!(entity_uuid.to_string()));
        entity
            .field_data
            .insert("name".to_string(), json!("Test User"));
        entity
            .field_data
            .insert("email".to_string(), json!("test@example.com"));
        entity.field_data.insert("age".to_string(), json!(30));
        entity.field_data.insert("active".to_string(), json!(true));
        entity.field_data.insert(
            "created_at".to_string(),
            json!(OffsetDateTime::now_utc().to_string()),
        );
        entity.field_data.insert(
            "updated_at".to_string(),
            json!(OffsetDateTime::now_utc().to_string()),
        );
        entity
            .field_data
            .insert("created_by".to_string(), json!(created_by.to_string()));
        entity
            .field_data
            .insert("updated_by".to_string(), json!(created_by.to_string()));
        entity
            .field_data
            .insert("published".to_string(), json!(true));
        entity.field_data.insert("version".to_string(), json!(1));
        entity
            .field_data
            .insert("path".to_string(), json!(format!("/{}/", entity_type)));

        entity
    }

    // Helper to get UUID from entity's field_data
    fn get_entity_uuid(entity: &DynamicEntity) -> Option<Uuid> {
        r_data_core::entity::dynamic_entity::utils::extract_uuid_from_entity_field_data(
            &entity.field_data,
            "uuid",
        )
    }

    /// Create a test entity definition from a JSON file with a unique entity type
    async fn create_test_entity_definition_from_json(
        pool: &PgPool,
        json_path: &str,
    ) -> Result<(String, EntityDefinition, Uuid)> {
        // Read the JSON file
        let json_content = std::fs::read_to_string(json_path).map_err(|e| {
            Error::Unknown(format!("Failed to read JSON file {}: {}", json_path, e))
        })?;

        // Parse the JSON into a EntityDefinition
        let mut entity_def: EntityDefinition =
            serde_json::from_str(&json_content).map_err(|e| {
                Error::Unknown(format!("Failed to parse JSON file {}: {}", json_path, e))
            })?;

        // Make the entity type unique to avoid test conflicts
        let unique_entity_type = unique_entity_type(&entity_def.entity_type);
        entity_def.entity_type = unique_entity_type.clone();

        // Ensure the entity definition is published
        entity_def.published = true;

        // Make sure we have a creator
        if entity_def.created_by == Uuid::nil() {
            entity_def.created_by = Uuid::now_v7();
        }

        // Create the entity definition using the service
        let entity_definition_repository = EntityDefinitionRepository::new(pool.clone());
        let entity_definition_service =
            EntityDefinitionService::new(Arc::new(entity_definition_repository));
        let uuid = entity_definition_service
            .create_entity_definition(&entity_def)
            .await?;

        // Wait a moment for the service to create the necessary database objects
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify the entity definition was saved as published with a direct database query
        let published = sqlx::query_scalar::<_, bool>(
            "SELECT published FROM entity_definitions WHERE uuid = $1",
        )
        .bind(uuid)
        .fetch_one(pool)
        .await
        .map_err(Error::Database)?;

        if !published {
            // If not published, update it directly
            sqlx::query("UPDATE entity_definitions SET published = true WHERE uuid = $1")
                .bind(uuid)
                .execute(pool)
                .await
                .map_err(Error::Database)?;

            // Wait a moment for the update to take effect
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Get the updated entity definition
        let saved_entity_def = entity_definition_service
            .get_entity_definition_by_entity_type(&unique_entity_type)
            .await?;

        Ok((unique_entity_type, saved_entity_def, uuid))
    }

    // Helper to ensure an entity is published
    async fn ensure_entity_published(pool: &PgPool, uuid: &Uuid, entity_type: &str) -> Result<()> {
        // Use a direct SQL query to ensure the entity is marked as published
        let result = sqlx::query(
            "UPDATE entities_registry SET published = true WHERE uuid = $1 AND entity_type = $2",
        )
        .bind(uuid)
        .bind(entity_type)
        .execute(pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            warn!("No rows were updated when marking entity as published");
        }

        // Give the database a moment to process the update
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    #[actix_web::test]
    async fn test_create_and_get_entity() -> Result<()> {
        // Setup for this test
        test_setup();

        // Setup test database
        let pool = common::utils::setup_test_db().await;

        // Create a unique entity type for this test
        let entity_type = unique_entity_type("user");

        // Create entity definition and get its content
        let (_, entity_def) = create_test_entity_definition(&pool, &entity_type).await?;

        // Create repository and service
        let repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let entity_definition_repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let class_service = EntityDefinitionService::new(entity_definition_repository);
        let service = DynamicEntityService::new(repository.clone(), Arc::new(class_service));

        // Create entity with the entity definition
        let test_entity = create_test_entity(&entity_type, Arc::new(entity_def), None);
        let test_uuid = get_entity_uuid(&test_entity).unwrap();

        // Create the entity using the service
        service.create_entity(&test_entity).await?;

        // Retrieve entity
        let retrieved = service
            .get_entity_by_uuid(&entity_type, &test_uuid, None)
            .await?;

        // Verify entity was retrieved correctly
        assert!(retrieved.is_some(), "Entity should be found");
        let retrieved = retrieved.unwrap();
        let retrieved_uuid = get_entity_uuid(&retrieved).unwrap();
        assert_eq!(retrieved_uuid, test_uuid, "UUIDs should match");
        assert_eq!(
            retrieved.entity_type, test_entity.entity_type,
            "Entity types should match"
        );
        assert_eq!(
            retrieved.field_data.get("name").unwrap(),
            &json!("Test User"),
            "Name field should match"
        );
        assert_eq!(
            retrieved.field_data.get("email").unwrap(),
            &json!("test@example.com"),
            "Email field should match"
        );

        // Clean up resources for the next test
        Ok(())
    }

    #[actix_web::test]
    async fn test_filter_entities() -> Result<()> {
        // Setup for this test
        test_setup();

        // Setup test database
        let pool = common::utils::setup_test_db().await;

        // Create a entity definition from the JSON example with a unique entity type
        let (entity_type, entity_def, _) = create_test_entity_definition_from_json(
            &pool,
            ".json_examples/user_entity_definition.json",
        )
        .await?;

        println!("Created entity type: {}", entity_type);

        // Create dynamic entity service
        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let class_service = EntityDefinitionService::new(repository);
        let dynamic_entity_service =
            DynamicEntityService::new(dynamic_entity_repository.clone(), Arc::new(class_service));

        // Create 5 test entities with different attributes
        let mut created_uuids = Vec::new();
        for i in 1..=5 {
            let mut entity = DynamicEntity {
                entity_type: entity_type.clone(),
                field_data: HashMap::new(),
                definition: Arc::new(entity_def.clone()),
            };

            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();

            // Use fields from the JSON definition
            entity
                .field_data
                .insert("uuid".to_string(), json!(uuid.to_string()));
            entity
                .field_data
                .insert("email".to_string(), json!(format!("user{}@example.com", i)));
            entity
                .field_data
                .insert("username".to_string(), json!(format!("user{}", i)));
            entity
                .field_data
                .insert("first_name".to_string(), json!(format!("User{}", i)));
            entity
                .field_data
                .insert("last_name".to_string(), json!(format!("Test{}", i)));
            entity.field_data.insert(
                "role".to_string(),
                json!(if i % 2 == 0 { "admin" } else { "customer" }),
            );
            entity.field_data.insert(
                "status".to_string(),
                json!(if i % 2 == 0 { "active" } else { "inactive" }),
            );
            entity
                .field_data
                .insert("newsletter_opt_in".to_string(), json!(true));
            entity
                .field_data
                .insert("created_by".to_string(), json!(created_by.to_string()));
            entity.field_data.insert(
                "created_at".to_string(),
                json!(OffsetDateTime::now_utc().to_string()),
            );
            entity.field_data.insert(
                "updated_at".to_string(),
                json!(OffsetDateTime::now_utc().to_string()),
            );
            entity
                .field_data
                .insert("published".to_string(), json!(true));
            entity.field_data.insert("version".to_string(), json!(1));

            // Insert using the service
            dynamic_entity_service.create_entity(&entity).await?;
            let entity_uuid = get_entity_uuid(&entity).unwrap();
            created_uuids.push(entity_uuid);

            // Ensure it's published
            ensure_entity_published(&pool, &entity_uuid, &entity_type).await?;

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Test filtering by role=admin
        let admin_entities = dynamic_entity_service
            .list_entities_with_filters(
                &entity_type,
                100,  // limit
                0,    // offset
                None, // fields
                None, // sort_by
                None, // sort_direction
                Some(json!({"role": "admin"})),
                None, // search_query
            )
            .await?;

        assert_eq!(
            admin_entities.0.len(),
            2,
            "Should have 2 admin users (indices 2 and 4)"
        );

        // Test filtering by status=active
        let active_entities = dynamic_entity_service
            .list_entities_with_filters(
                &entity_type,
                100,  // limit
                0,    // offset
                None, // fields
                None, // sort_by
                None, // sort_direction
                Some(json!({"status": "active"})),
                None, // search_query
            )
            .await?;

        assert_eq!(
            active_entities.0.len(),
            2,
            "Should have 2 active users (indices 2 and 4)"
        );

        Ok(())
    }

    #[actix_web::test]
    async fn test_get_entities_with_pagination() -> Result<()> {
        // Setup for this test
        test_setup();

        // Setup test database
        let pool = common::utils::setup_test_db().await;

        // Create a entity definition from the JSON example with a unique entity type
        let (entity_type, entity_def, _) = create_test_entity_definition_from_json(
            &pool,
            ".json_examples/user_entity_definition.json",
        )
        .await?;

        // Create dynamic entity service
        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let class_service = EntityDefinitionService::new(repository);
        let dynamic_entity_service =
            DynamicEntityService::new(dynamic_entity_repository.clone(), Arc::new(class_service));

        // Create 3 test entities using JSON-based fields
        for i in 1..=3 {
            let mut entity = DynamicEntity {
                entity_type: entity_type.clone(),
                field_data: HashMap::new(),
                definition: Arc::new(entity_def.clone()),
            };

            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();
            entity
                .field_data
                .insert("uuid".to_string(), json!(uuid.to_string()));
            entity
                .field_data
                .insert("email".to_string(), json!(format!("user{}@example.com", i)));
            entity
                .field_data
                .insert("username".to_string(), json!(format!("user{}", i)));
            entity
                .field_data
                .insert("first_name".to_string(), json!(format!("User{}", i)));
            entity
                .field_data
                .insert("last_name".to_string(), json!(format!("Test{}", i)));
            entity
                .field_data
                .insert("role".to_string(), json!("customer"));
            entity
                .field_data
                .insert("status".to_string(), json!("active"));
            entity
                .field_data
                .insert("newsletter_opt_in".to_string(), json!(true));
            entity
                .field_data
                .insert("created_by".to_string(), json!(created_by.to_string()));

            // Create the entity
            dynamic_entity_service.create_entity(&entity).await?;
        }

        // Test pagination - first page
        let first_page = dynamic_entity_service
            .list_entities_with_filters(
                &entity_type,
                2,    // limit
                0,    // offset
                None, // fields
                None, // sort_by
                None, // sort_direction
                None, // filter
                None, // search_query
            )
            .await?;

        assert_eq!(first_page.0.len(), 2, "First page should have 2 items");

        // Test pagination - second page
        let second_page = dynamic_entity_service
            .list_entities_with_filters(
                &entity_type,
                2,    // limit
                2,    // offset
                None, // fields
                None, // sort_by
                None, // sort_direction
                None, // filter
                None, // search_query
            )
            .await?;

        assert_eq!(
            second_page.0.len(),
            1,
            "Second page should have 1 item (3 total - 2 from first page)"
        );

        Ok(())
    }

    #[actix_web::test]
    async fn test_dynamic_entity_uuid_access() -> Result<()> {
        // Setup for this test
        test_setup();

        // This test doesn't need a database, continue with the rest
        // Create a test entity
        let mut entity = DynamicEntity {
            entity_type: "test".to_string(),
            field_data: HashMap::new(),
            definition: Arc::new(EntityDefinition::default()),
        };

        // Add a UUID field
        let test_uuid = Uuid::now_v7();
        entity
            .field_data
            .insert("uuid".to_string(), json!(test_uuid.to_string()));

        // Test our UUID extraction function
        let extracted_uuid = get_entity_uuid(&entity);
        assert!(extracted_uuid.is_some(), "Should be able to extract UUID");
        assert_eq!(
            extracted_uuid.unwrap(),
            test_uuid,
            "Extracted UUID should match"
        );

        Ok(())
    }

    #[actix_web::test]
    async fn test_dynamic_entity_pagination() -> Result<()> {
        // Setup for this test
        test_setup();

        // Setup test database
        let pool = common::utils::setup_test_db().await;

        // Create a entity definition from the JSON example with a unique entity type
        let (entity_type, entity_def, _) = create_test_entity_definition_from_json(
            &pool,
            ".json_examples/user_entity_definition.json",
        )
        .await?;

        println!("Testing pagination with entity type: {}", entity_type);

        // Create repositories and services for this test
        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let class_service = EntityDefinitionService::new(repository);
        let dynamic_entity_service =
            DynamicEntityService::new(dynamic_entity_repository.clone(), Arc::new(class_service));

        // Create just 3 test entities (simplified)
        for i in 1..=3 {
            let mut entity = DynamicEntity {
                entity_type: entity_type.clone(),
                field_data: HashMap::new(),
                definition: Arc::new(entity_def.clone()),
            };

            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();
            entity
                .field_data
                .insert("uuid".to_string(), json!(uuid.to_string()));
            entity
                .field_data
                .insert("email".to_string(), json!(format!("user{}@example.com", i)));
            entity
                .field_data
                .insert("username".to_string(), json!(format!("user{}", i)));
            entity
                .field_data
                .insert("first_name".to_string(), json!(format!("User{}", i)));
            entity
                .field_data
                .insert("last_name".to_string(), json!(format!("Test{}", i)));
            entity
                .field_data
                .insert("role".to_string(), json!("customer"));
            entity
                .field_data
                .insert("status".to_string(), json!("active"));
            entity
                .field_data
                .insert("newsletter_opt_in".to_string(), json!(true));
            entity
                .field_data
                .insert("created_by".to_string(), json!(created_by.to_string()));

            // Create the entity
            dynamic_entity_service.create_entity(&entity).await?;
        }

        // Test pagination - first page with limit 2
        let first_page = dynamic_entity_service
            .list_entities(
                &entity_type,
                2,    // limit
                0,    // offset
                None, // exclusive_fields
            )
            .await?;

        assert_eq!(first_page.len(), 2, "First page should have 2 entities");

        // Test pagination - second page
        let second_page = dynamic_entity_service
            .list_entities(
                &entity_type,
                2,    // limit
                2,    // offset
                None, // exclusive_fields
            )
            .await?;

        assert_eq!(
            second_page.len(),
            1,
            "Second page should have 1 entity (3 total - 2 from first page)"
        );

        Ok(())
    }

    #[actix_web::test]
    async fn test_fixed_entity_type_column_issue() -> Result<()> {
        // Setup for this test
        test_setup();

        // Setup test database
        let pool = common::utils::setup_test_db().await;

        // Create a entity definition from the JSON example with a unique entity type
        let (entity_type, entity_def, _) = create_test_entity_definition_from_json(
            &pool,
            ".json_examples/user_entity_definition.json",
        )
        .await?;

        println!("Created entity type: {}", entity_type);

        // Create dynamic entity service
        let dynamic_entity_repository = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
        let class_service = EntityDefinitionService::new(repository);
        let dynamic_entity_service =
            DynamicEntityService::new(dynamic_entity_repository, Arc::new(class_service));

        // Create a few test entities
        let mut created_uuids = Vec::new();
        for i in 1..=3 {
            let mut entity = DynamicEntity {
                entity_type: entity_type.clone(),
                field_data: HashMap::new(),
                definition: Arc::new(entity_def.clone()),
            };

            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();
            entity
                .field_data
                .insert("uuid".to_string(), json!(uuid.to_string()));
            entity
                .field_data
                .insert("email".to_string(), json!(format!("user{}@example.com", i)));
            entity
                .field_data
                .insert("username".to_string(), json!(format!("user{}", i)));
            entity
                .field_data
                .insert("first_name".to_string(), json!(format!("User{}", i)));
            entity
                .field_data
                .insert("last_name".to_string(), json!(format!("Test{}", i)));
            entity
                .field_data
                .insert("role".to_string(), json!("customer"));
            entity
                .field_data
                .insert("status".to_string(), json!("active"));
            entity
                .field_data
                .insert("newsletter_opt_in".to_string(), json!(true));
            entity
                .field_data
                .insert("created_by".to_string(), json!(created_by.to_string()));
            entity.field_data.insert(
                "created_at".to_string(),
                json!(OffsetDateTime::now_utc().to_string()),
            );
            entity.field_data.insert(
                "updated_at".to_string(),
                json!(OffsetDateTime::now_utc().to_string()),
            );
            entity
                .field_data
                .insert("published".to_string(), json!(true));
            entity.field_data.insert("version".to_string(), json!(1));

            // Insert using the service
            dynamic_entity_service.create_entity(&entity).await?;
            let entity_uuid = get_entity_uuid(&entity).unwrap();
            created_uuids.push(entity_uuid);

            // Ensure it's published
            ensure_entity_published(&pool, &entity_uuid, &entity_type).await?;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Test listing all entities
        let entities = dynamic_entity_service
            .list_entities(
                &entity_type,
                100,  // limit
                0,    // offset
                None, // exclusive_fields
            )
            .await?;

        // Verify expected count of entities
        assert_eq!(entities.len(), 3, "Should have created 3 entities");

        // Verify entity_type field is set correctly in all entities
        for entity in &entities {
            assert_eq!(entity.entity_type, entity_type, "Entity type should match");
        }

        // Clean up all test resources at the end of tests
        common::utils::cleanup_test_resources().await?;

        Ok(())
    }

    #[actix_web::test]
    async fn test_simple_entity_creation_optimized() -> Result<()> {
        // Set up the test database
        let pool = common::utils::setup_test_db().await;

        // Create a simple entity definition
        let entity_type = format!("test_entity_{}", Uuid::now_v7().simple());

        // Create a simple entity definition
        let mut entity_def = EntityDefinition::default();
        entity_def.entity_type = entity_type.clone();
        entity_def.display_name = format!("Test {}", entity_type);
        entity_def.description = Some("Test entity".to_string());
        entity_def.created_by = Uuid::now_v7();
        entity_def.published = true;

        // Create a simple field definition
        let mut fields = Vec::new();
        let field = r_data_core::entity::field::FieldDefinition::new(
            "name".to_string(),
            "Name".to_string(),
            r_data_core::entity::field::types::FieldType::String,
        );
        fields.push(field);
        entity_def.fields = fields;

        // Create entity definition in the database
        let class_repo = EntityDefinitionRepository::new(pool.clone());
        let class_service = EntityDefinitionService::new(Arc::new(class_repo));
        let entity_uuid = class_service.create_entity_definition(&entity_def).await?;

        // Wait for database operations to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Get the entity definition
        let entity_def = class_service.get_entity_definition(&entity_uuid).await?;

        // Create a dynamic entity
        let mut entity = DynamicEntity {
            entity_type: entity_type.clone(),
            field_data: HashMap::new(),
            definition: Arc::new(entity_def),
        };

        // Set entity fields
        let uuid = Uuid::now_v7();
        entity
            .field_data
            .insert("uuid".to_string(), json!(uuid.to_string()));
        entity
            .field_data
            .insert("name".to_string(), json!("Test Entity"));
        entity
            .field_data
            .insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));
        entity.field_data.insert(
            "created_at".to_string(),
            json!(OffsetDateTime::now_utc().to_string()),
        );
        entity.field_data.insert(
            "updated_at".to_string(),
            json!(OffsetDateTime::now_utc().to_string()),
        );
        entity
            .field_data
            .insert("published".to_string(), json!(true));
        entity.field_data.insert("version".to_string(), json!(1));

        // Create a repository and service for dynamic entities
        let entity_repo = Arc::new(DynamicEntityRepository::new(pool.clone()));
        let dynamic_service = DynamicEntityService::new(entity_repo, Arc::new(class_service));

        // Create the entity
        dynamic_service.create_entity(&entity).await?;

        // Retrieve the entity
        let entity_uuid = Uuid::parse_str(&entity.field_data["uuid"].as_str().unwrap())
            .map_err(|e| r_data_core::error::Error::Conversion(e.to_string()))?;
        let retrieved = dynamic_service
            .get_entity_by_uuid(&entity_type, &entity_uuid, None)
            .await?;

        // Verify entity was retrieved correctly
        assert!(retrieved.is_some(), "Entity should exist");
        assert_eq!(
            retrieved.unwrap().field_data.get("name").unwrap(),
            &json!("Test Entity"),
            "Name should match"
        );

        Ok(())
    }
}
