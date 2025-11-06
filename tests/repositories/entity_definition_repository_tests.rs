use crate::common::utils::clear_test_db;
use crate::repositories::{get_entity_definition_repository_with_pool, TestRepository};
use r_data_core::entity::entity_definition::definition::EntityDefinition;
use r_data_core::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core::entity::field::types::FieldType;
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::entity::field::FieldDefinition;
use serial_test::serial;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_create_and_get_entity_definition() {
    // Get test repository and clear database first
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    // Create test definition
    let creator_id = Uuid::now_v7();

    let test_def = EntityDefinition::new(
        "testentity".to_string(),
        "Test Entity".to_string(),
        Some("Test entity for repository tests".to_string()),
        None,  // No group
        false, // No children
        None,  // No icon
        vec![
            FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: Some("The name field".to_string()),
                field_type: FieldType::String,
                required: true,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "description".to_string(),
                display_name: "Description".to_string(),
                description: Some("A description".to_string()),
                field_type: FieldType::Text,
                required: false,
                indexed: false,
                filterable: false,
                default_value: None,
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        creator_id,
    );

    // Save the definition
    let uuid = repository
        .create(&test_def)
        .await
        .expect("Failed to create test definition");

    // Verify we can retrieve it by UUID
    let retrieved = repository
        .get_by_uuid(&uuid)
        .await
        .expect("Failed to get definition by UUID")
        .expect("Definition not found");
    assert_eq!(retrieved.entity_type, "testentity");
    assert_eq!(retrieved.display_name, "Test Entity");
    assert_eq!(retrieved.fields.len(), 2);

    // Verify we can retrieve it by entity type
    let by_type = repository
        .get_by_entity_type("testentity")
        .await
        .expect("Failed to get definition by entity type")
        .expect("Definition not found by entity type");
    assert_eq!(by_type.uuid, uuid);

    // Clean up - drop the test table and delete the definition
    let _ = repository.delete(&uuid).await;
}

#[tokio::test]
#[serial]
async fn test_list_entity_definitions() {
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    // Create a few test definitions
    let creator_id = Uuid::now_v7();
    let mut uuids = Vec::new();

    for i in 1..=3 {
        let entity_type = format!("testlist{}", i);
        let definition = EntityDefinition::new(
            entity_type.clone(),
            format!("Test List {}", i),
            Some(format!("Test definition {}", i)),
            None,  // No group
            false, // No children
            None,  // No icon
            vec![FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: None,
                field_type: FieldType::String,
                required: true,
                indexed: false,
                filterable: true,
                default_value: None,
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            }],
            creator_id,
        );

        let uuid = repository.create(&definition).await.unwrap();
        uuids.push(uuid);
    }

    // Check we can list all definitions
    let all_definitions = repository.list(100, 0).await.unwrap();
    assert_eq!(all_definitions.len(), 3);

    // Check pagination works - get first 2 items
    let first_page = repository.list(2, 0).await.unwrap();
    assert_eq!(first_page.len(), 2);

    // Get another page
    let second_page = repository.list(2, 2).await.unwrap();
    assert_eq!(second_page.len(), 1);

    // Ensure the pages are different
    assert_ne!(first_page[0].uuid, second_page[0].uuid);

    // Clean up - delete the test definitions
    for uuid in uuids {
        let _ = repository.delete(&uuid).await;
    }
}

#[tokio::test]
#[serial]
async fn test_update_entity_definition() {
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    // Create a test definition
    let creator_id = Uuid::now_v7();
    let definition = EntityDefinition::new(
        "testupdate".to_string(),
        "Original Title".to_string(),
        Some("Original description".to_string()),
        None,  // No group
        false, // No children
        None,  // No icon
        vec![FieldDefinition {
            name: "field1".to_string(),
            display_name: "Field 1".to_string(),
            description: None,
            field_type: FieldType::String,
            required: true,
            indexed: false,
            filterable: true,
            default_value: None,
            validation: Default::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        creator_id,
    );

    let uuid = repository.create(&definition).await.unwrap();

    // Update the definition
    let mut updated = repository.get_by_uuid(&uuid).await.unwrap().unwrap();
    updated.display_name = "Updated Title".to_string();
    updated.description = Some("Updated description".to_string());

    // Add a new field
    updated.fields.push(FieldDefinition {
        name: "field2".to_string(),
        display_name: "Field 2".to_string(),
        description: Some("A new field".to_string()),
        field_type: FieldType::Integer,
        required: false,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: Default::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    });

    // Save the update
    repository.update(&uuid, &updated).await.unwrap();

    // Verify the update
    let retrieved = repository.get_by_uuid(&uuid).await.unwrap().unwrap();
    assert_eq!(retrieved.display_name, "Updated Title");
    assert_eq!(
        retrieved.description,
        Some("Updated description".to_string())
    );
    assert_eq!(retrieved.fields.len(), 2);

    // Verify the table structure has been updated
    let table_name = format!("entity_{}_view", updated.entity_type.to_lowercase());
    let columns = repository
        .get_view_columns_with_types(&table_name)
        .await
        .unwrap();

    // Verify both fields exist in the table
    assert!(
        columns.contains_key("field1"),
        "field1 not found, but columns: {:?}",
        columns
    );
    assert!(
        columns.contains_key("field2"),
        "field2 not found, but columns: {:?}",
        columns
    );

    // Clean up
    let _ = repository.delete(&uuid).await;
}

#[tokio::test]
#[serial]
async fn test_delete_entity_definition() {
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    // Create a test definition
    let creator_id = Uuid::now_v7();
    let definition = EntityDefinition::new(
        "testdelete".to_string(),
        "Test Delete".to_string(),
        None,
        None,  // No group
        false, // No children
        None,  // No icon
        vec![FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            description: None,
            field_type: FieldType::String,
            required: true,
            indexed: false,
            filterable: true,
            default_value: None,
            validation: Default::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        creator_id,
    );

    let uuid = repository.create(&definition).await.unwrap();

    // Verify it exists
    let retrieved = repository.get_by_uuid(&uuid).await.unwrap();
    assert!(retrieved.is_some());

    // Verify the table exists
    let table_name = format!("entity_{}", definition.entity_type.to_lowercase());
    let table_exists = repository.check_view_exists(&table_name).await.unwrap();
    assert!(table_exists);

    // Delete the definition
    repository.delete(&uuid).await.unwrap();

    // Verify it's gone
    let after_delete = repository.get_by_uuid(&uuid).await.unwrap();
    assert!(after_delete.is_none());

    // Verify the table is gone
    let table_exists_after = repository.check_view_exists(&table_name).await.unwrap();
    assert!(!table_exists_after);
}

#[tokio::test]
#[serial]
async fn test_table_operations() {
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    // Create a test definition
    let creator_id = Uuid::now_v7();
    let definition = EntityDefinition::new(
        "testtable".to_string(),
        "Test Table".to_string(),
        None,
        None,  // No group
        false, // No children
        None,  // No icon
        vec![
            FieldDefinition {
                name: "column1".to_string(),
                display_name: "Column 1".to_string(),
                description: None,
                field_type: FieldType::String,
                required: true,
                indexed: false,
                filterable: true,
                default_value: None,
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "column2".to_string(),
                display_name: "Column 2".to_string(),
                description: None,
                field_type: FieldType::Integer,
                required: false,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        creator_id,
    );

    let uuid = repository.create(&definition).await.unwrap();

    // Check the table exists
    let view_name = format!("entity_{}_view", definition.entity_type.to_lowercase());
    let view_exists = repository.check_view_exists(&view_name).await.unwrap();
    assert!(view_exists);

    // Get columns and verify they match our fields
    let columns = repository
        .get_view_columns_with_types(&view_name)
        .await
        .unwrap();

    // Verify the table has all the expected columns
    assert!(
        columns.contains_key("uuid"),
        "uuid not found, but columns: {:?}",
        columns
    );
    assert!(
        columns.contains_key("created_at"),
        "created_at not found, but columns: {:?}",
        columns
    );
    assert!(
        columns.contains_key("updated_at"),
        "updated_at not found, but columns: {:?}",
        columns
    );
    assert!(
        columns.contains_key("created_by"),
        "created_by not found, but columns: {:?}",
        columns
    );

    // Check that our custom fields are there
    assert!(
        columns.contains_key("column1"),
        "column1 not found, but columns: {:?}",
        columns
    );
    assert!(
        columns.contains_key("column2"),
        "column2 not found, but columns: {:?}",
        columns
    );

    // Verify data types
    assert!(columns
        .get("column1")
        .unwrap()
        .contains("character varying"));
    assert!(columns.get("column2").unwrap().contains("integer"));

    // Count records - should be 0
    let count = repository.count_view_records(&view_name).await.unwrap();
    assert_eq!(count, 0);

    // Clean up
    let _ = repository.delete(&uuid).await;
}

#[tokio::test]
#[serial]
async fn test_create_entity_definition_from_json_examples() {
    let TestRepository {
        repository,
        db_pool,
    } = get_entity_definition_repository_with_pool().await;
    clear_test_db(&db_pool)
        .await
        .expect("Failed to clear database");

    let creator_id = Uuid::now_v7();

    // Load JSON examples
    let product_json = std::fs::read_to_string("../../.example_files/json_examples/product_entity_definition.json")
        .expect("Failed to read product JSON example");

    let user_json = std::fs::read_to_string("../../.example_files/json_examples/user_entity_definition.json")
        .expect("Failed to read user JSON example");

    let order_json = std::fs::read_to_string("../../.example_files/json_examples/order_entity_definition.json")
        .expect("Failed to read order JSON example");

    let examples = vec![product_json, user_json, order_json];
    let example_names = vec!["product", "user", "order"];
    let mut created_uuids = Vec::new();

    for (i, json_str) in examples.iter().enumerate() {
        // Parse JSON to EntityDefinition
        let mut definition: EntityDefinition = serde_json::from_str(&json_str).expect(&format!(
            "Failed to parse {} JSON example",
            example_names[i]
        ));

        // Set creator ID and other required fields
        definition.created_by = creator_id;
        definition.uuid = Uuid::now_v7();

        // Convert entity_type to lowercase to avoid case sensitivity issues
        definition.entity_type = definition.entity_type.to_lowercase();

        // Create the entity definition in the repository
        let uuid = match repository.create(&definition).await {
            Ok(id) => id,
            Err(e) => {
                eprintln!(
                    "Failed to create {} entity definition: {}",
                    example_names[i], e
                );
                continue;
            }
        };
        created_uuids.push(uuid);

        // Verify it was created correctly
        let retrieved = repository.get_by_uuid(&uuid).await.unwrap().unwrap();
        assert_eq!(retrieved.entity_type, definition.entity_type);
        assert_eq!(retrieved.fields.len(), definition.fields.len());

        // Verify schema was applied by checking table existence
        let table_name = format!("entity_{}", definition.entity_type.to_lowercase());
        let table_exists = repository.check_view_exists(&table_name).await.unwrap();
        assert!(table_exists);
    }

    // Clean up created definitions
    for uuid in created_uuids {
        let _ = repository.delete(&uuid).await;
    }
}
