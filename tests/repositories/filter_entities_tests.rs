#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{
    DynamicEntityRepository, DynamicEntityRepositoryTrait, FilterEntitiesParams,
};

use r_data_core_test_support::{clear_test_db, setup_test_db};

#[cfg(test)]
#[allow(clippy::module_inception)]
mod filter_entities_tests {
    use super::*;

    // Helper to create a test entity definition
    async fn create_test_entity_definition(db_pool: &PgPool, entity_type: &str) -> Result<Uuid> {
        // Create a simple entity definition for testing
        let mut entity_def = EntityDefinition {
            entity_type: entity_type.to_string(),
            display_name: format!("Test {entity_type}"),
            description: Some(format!("Test description for {entity_type}")),
            published: true,
            ..Default::default()
        };

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
            .map_err(r_data_core_core::error::Error::from)?;

        // Execute SQL to trigger the view creation directly
        let trigger_sql = format!(
            "SELECT create_entity_table_and_view('{}')",
            entity_def.entity_type
        );

        sqlx::query(&trigger_sql)
            .execute(db_pool)
            .await
            .map_err(r_data_core_core::error::Error::from)?;

        // Wait a moment for the trigger to create the view
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(uuid)
    }

    // Helper to create test entities
    async fn create_test_entities(
        db_pool: &PgPool,
        entity_type: &str,
        count: i32,
    ) -> Result<Vec<Uuid>> {
        let mut uuids = Vec::new();
        let repository = DynamicEntityRepository::new(db_pool.clone());

        for i in 1..=count {
            let uuid = Uuid::now_v7();
            let created_by = Uuid::now_v7();

            let mut field_data = HashMap::new();
            field_data.insert("uuid".to_string(), json!(uuid.to_string()));
            field_data.insert("entity_key".to_string(), json!(format!("test-entity-{i}")));
            field_data.insert("name".to_string(), json!(format!("Test Entity {i}")));
            field_data.insert("email".to_string(), json!(format!("test{i}@example.com")));
            field_data.insert("age".to_string(), json!(20 + i));
            field_data.insert("active".to_string(), json!(i % 2 == 0));
            field_data.insert("created_by".to_string(), json!(created_by.to_string()));

            let entity = DynamicEntity {
                entity_type: entity_type.to_string(),
                field_data,
                definition: Arc::new(EntityDefinition::default()),
            };

            repository.create(&entity).await?;
            uuids.push(uuid);
        }

        Ok(uuids)
    }

    // Ensure we use a unique entity type for each test to avoid conflicts
    fn unique_entity_type(base: &str) -> String {
        format!("{base}_{}", Uuid::now_v7().simple())
    }

    #[tokio::test]
    async fn test_filter_entities_by_active() -> Result<()> {
        // Setup database
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;

        // Filter by active = true
        let mut filters = HashMap::new();
        filters.insert("active".to_string(), json!(true));

        let params = FilterEntitiesParams::new(100, 0).with_filters(Some(filters));
        let active_entities = repository.filter_entities(&entity_type, &params).await?;

        // We should have about 10 active entities (even numbers)
        assert!(
            active_entities.len() >= 9 && active_entities.len() <= 11,
            "Should retrieve about 10 active entities (got {})",
            active_entities.len()
        );

        // Verify all retrieved entities are active
        for entity in &active_entities {
            let active_value = entity.field_data.get("active").unwrap();
            assert!(
                active_value.as_bool().unwrap(),
                "All filtered entities should be active"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_filter_entities_by_age() -> Result<()> {
        // Setup database
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;

        // Filter by age = 30
        let mut filters = HashMap::new();
        filters.insert("age".to_string(), json!(30));

        let params = FilterEntitiesParams::new(100, 0).with_filters(Some(filters));
        let filtered_entities = repository.filter_entities(&entity_type, &params).await?;

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
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;

        // Search for entities with "Test Entity 1" in name
        let params = FilterEntitiesParams::new(100, 0).with_search(Some((
            "Test Entity 1".to_string(),
            vec!["name".to_string()],
        )));
        let search_result = repository.filter_entities(&entity_type, &params).await?;

        // Should find entities with "Test Entity 1" in the name (1, 10-19)
        assert!(
            !search_result.is_empty(),
            "Should find at least one entity with search term"
        );

        for entity in &search_result {
            let name = entity.field_data.get("name").unwrap().as_str().unwrap();
            assert!(
                name.contains("Test Entity 1"),
                "Search results should match search term"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_filter_entities_with_sort() -> Result<()> {
        // Setup database
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;

        // Get entities sorted by age ascending
        let params = FilterEntitiesParams::new(100, 0)
            .with_sort(Some(("age".to_string(), "ASC".to_string())));
        let sorted_entities = repository.filter_entities(&entity_type, &params).await?;

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
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 20 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 20).await?;

        // Get entities with only name field
        let params = FilterEntitiesParams::new(100, 0).with_fields(Some(vec!["name".to_string()]));
        let entities = repository.filter_entities(&entity_type, &params).await?;

        // Verify entities have name field but not age or active fields
        for entity in &entities {
            assert!(
                entity.field_data.contains_key("name"),
                "Name field should be present"
            );
            assert!(
                !entity.field_data.contains_key("age"),
                "Age field should not be present"
            );
            assert!(
                !entity.field_data.contains_key("active"),
                "Active field should not be present"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_filter_entities_with_pagination() -> Result<()> {
        // Setup database
        let db_pool = setup_test_db().await;
        clear_test_db(&db_pool)
            .await
            .expect("Failed to clear test database");

        // Create a test entity definition with a unique entity type
        let entity_type = unique_entity_type("testentity");
        let _entity_uuid = create_test_entity_definition(&db_pool, &entity_type).await?;

        // Create repository
        let repository = DynamicEntityRepository::new(db_pool.pool.clone());

        // Create 30 test entities
        let _uuids = create_test_entities(&db_pool, &entity_type, 30).await?;

        // Get first page (10 entities)
        let params1 = FilterEntitiesParams::new(10, 0);
        let page1 = repository.filter_entities(&entity_type, &params1).await?;

        // Get second page (10 entities)
        let params2 = FilterEntitiesParams::new(10, 10);
        let page2 = repository.filter_entities(&entity_type, &params2).await?;

        // Test pagination works as expected
        assert_eq!(page1.len(), 10, "First page should have 10 entities");
        assert_eq!(page2.len(), 10, "Second page should have 10 entities");

        // Make sure the pages contain different entities
        let page1_names: Vec<String> = page1
            .iter()
            .map(|e| {
                e.field_data
                    .get("name")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();

        let page2_names: Vec<String> = page2
            .iter()
            .map(|e| {
                e.field_data
                    .get("name")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
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
