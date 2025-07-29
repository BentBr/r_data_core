use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core::entity::class::definition::ClassDefinition;
use r_data_core::entity::dynamic_entity::entity::DynamicEntity;
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::error::Result;

// Import common module for test setup
#[path = "../common/mod.rs"]
mod common;

#[cfg(test)]
mod filter_entities_tests {
    use super::*;

    // Helper to create a test class definition
    async fn create_test_class_definition(db_pool: &PgPool, entity_type: &str) -> Result<Uuid> {
        // Create a simple class definition for testing
        let mut class_def = ClassDefinition::default();
        class_def.entity_type = entity_type.to_string();
        class_def.display_name = format!("Test {}", entity_type);
        class_def.description = Some(format!("Test description for {}", entity_type));
        class_def.published = true;
        
        // Add fields to the class definition
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
        
        // Email field (additional field)
        let email_field = FieldDefinition {
            name: "email".to_string(),
            display_name: "Email".to_string(),
            field_type: FieldType::String,
            required: false,
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
        
        class_def.fields = fields;
        
        // Save to database
        let create_query = 
            "INSERT INTO class_definitions (uuid, entity_type, display_name, description, field_definitions, created_at, created_by, published) 
             VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7) RETURNING uuid";
        
        let uuid = Uuid::now_v7();
        let created_by = Uuid::now_v7();
        
        sqlx::query(create_query)
            .bind(uuid)
            .bind(&class_def.entity_type)
            .bind(&class_def.display_name)
            .bind(&class_def.description)
            .bind(json!(class_def.fields))
            .bind(created_by)
            .bind(class_def.published)
            .fetch_one(db_pool)
            .await
            .map_err(|e| r_data_core::error::Error::Database(e))?;
            
        // Execute SQL to trigger the view creation directly
        let trigger_sql = format!(
            "SELECT create_entity_table_and_view('{}')",
            class_def.entity_type
        );
        
        sqlx::query(&trigger_sql)
            .execute(db_pool)
            .await
            .map_err(|e| r_data_core::error::Error::Database(e))?;
            
        // Wait a moment for the trigger to create the view
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(uuid)
    }
    
    // Helper to create test entities
    async fn create_test_entities(db_pool: &PgPool, entity_type: &str, count: i32) -> Result<Vec<Uuid>> {
        let mut uuids = Vec::new();
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        for i in 1..=count {
            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();
            
            let mut field_data = HashMap::new();
            field_data.insert("uuid".to_string(), json!(uuid.to_string()));
            field_data.insert("name".to_string(), json!(format!("Test Entity {}", i)));
            field_data.insert("email".to_string(), json!(format!("test{}@example.com", i)));
            field_data.insert("age".to_string(), json!(20 + i));
            field_data.insert("active".to_string(), json!(i % 2 == 0));
            field_data.insert("created_by".to_string(), json!(created_by.to_string()));
            
            let entity = DynamicEntity {
                entity_type: entity_type.to_string(),
                field_data,
                definition: Arc::new(ClassDefinition::default()),
            };
            
            repository.create(&entity).await?;
            uuids.push(uuid);
        }
        
        Ok(uuids)
    }

    // Ensure we use a unique entity type for each test to avoid conflicts
    fn unique_entity_type(base: &str) -> String {
        format!("{}_{}", base, Uuid::now_v7().simple())
    }

    #[tokio::test]
    async fn test_filter_entities_by_active() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;
        
        // Filter by active = true
        let mut filters = HashMap::new();
        filters.insert("active".to_string(), json!(true));
        
        let active_entities = repository.filter_entities(
            &entity_type,
            100,
            0,
            Some(filters),
            None,
            None,
            None
        ).await?;
        
        // We should have about 10 active entities (even numbers)
        assert!(active_entities.len() >= 9 && active_entities.len() <= 11, 
            "Should retrieve about 10 active entities (got {})", active_entities.len());
            
        // Verify all retrieved entities are active
        for entity in &active_entities {
            let active_value = entity.field_data.get("active").unwrap();
            assert_eq!(
                active_value.as_bool().unwrap(),
                true,
                "All filtered entities should be active"
            );
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filter_entities_by_age() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;
        
        // Filter by age = 30
        let mut filters = HashMap::new();
        filters.insert("age".to_string(), json!(30));
        
        let filtered_entities = repository.filter_entities(
            &entity_type,
            100,
            0,
            Some(filters),
            None,
            None,
            None
        ).await?;
        
        // Verify all retrieved entities have age = 30
        for entity in &filtered_entities {
            let age_value = entity.field_data.get("age").unwrap();
            assert_eq!(
                age_value.as_i64().unwrap(),
                30,
                "All filtered entities should have age = 30"
            );
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filter_entities_with_search() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;
        
        // Search for entities with "Test Entity 1" in name
        let search_result = repository.filter_entities(
            &entity_type,
            100,
            0,
            None,
            Some(("Test Entity 1".to_string(), vec!["name".to_string()])),
            None,
            None
        ).await?;
        
        // Should find entities with "Test Entity 1" in the name (1, 10-19)
        assert!(search_result.len() >= 1, "Should find at least one entity with search term");
        
        for entity in &search_result {
            let name = entity.field_data.get("name").unwrap().as_str().unwrap();
            assert!(name.contains("Test Entity 1"), "Search results should match search term");
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filter_entities_with_sort() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;
        
        // Get entities sorted by age ascending
        let sorted_entities = repository.filter_entities(
            &entity_type,
            100,
            0,
            None,
            None,
            Some(("age".to_string(), "ASC".to_string())),
            None
        ).await?;
        
        // Verify entities are sorted by age
        let mut prev_age = 0;
        for entity in &sorted_entities {
            let current_age = entity.field_data.get("age").unwrap().as_i64().unwrap();
            assert!(
                current_age >= prev_age, 
                "Entities should be sorted by age in ascending order"
            );
            prev_age = current_age;
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filter_entities_with_field_selection() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;
        
        // Get entities with only name field
        let entities = repository.filter_entities(
            &entity_type,
            100,
            0,
            None,
            None,
            None,
            Some(vec!["name".to_string()])
        ).await?;
        
        // Verify entities have name field but not age or active fields
        for entity in &entities {
            assert!(entity.field_data.contains_key("name"), "Name field should be present");
            assert!(!entity.field_data.contains_key("age"), "Age field should not be present");
            assert!(!entity.field_data.contains_key("active"), "Active field should not be present");
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filter_entities_with_pagination() -> Result<()> {
        // Setup database
        let db_pool = common::setup_test_db().await;
        common::clear_test_db(&db_pool).await.expect("Failed to clear test database");
        
        // Create a test class definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _class_uuid = create_test_class_definition(&db_pool, &entity_type).await?;
        
        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.clone());
        
        // Create 30 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 30).await?;
        
        // Get first page (10 entities)
        let page1 = repository.filter_entities(
            &entity_type,
            10,
            0,
            None,
            None,
            None,
            None
        ).await?;
        
        // Get second page (10 entities)
        let page2 = repository.filter_entities(
            &entity_type,
            10,
            10,
            None,
            None,
            None,
            None
        ).await?;
        
        // Test pagination works as expected
        assert_eq!(page1.len(), 10, "First page should have 10 entities");
        assert_eq!(page2.len(), 10, "Second page should have 10 entities");
        
        // Make sure the pages contain different entities
        let page1_names: Vec<String> = page1
            .iter()
            .map(|e| e.field_data.get("name").unwrap().as_str().unwrap().to_string())
            .collect();
            
        let page2_names: Vec<String> = page2
            .iter()
            .map(|e| e.field_data.get("name").unwrap().as_str().unwrap().to_string())
            .collect();
            
        // Check if any names appear in both pages
        let mut duplicate_found = false;
        for name in &page1_names {
            if page2_names.contains(name) {
                duplicate_found = true;
                break;
            }
        }
        
        assert!(!duplicate_found, "Pages should contain different entities");
        
        Ok(())
    }
} 