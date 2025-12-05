#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::*;
use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::{self, eq};
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::schema::Schema;
use r_data_core_core::error::Result;
use r_data_core_core::field::types::FieldType;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldValidation};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

// Create a mock for the repository trait
mock! {
    pub EntityDefinitionRepo {}

    #[async_trait]
    impl r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait for EntityDefinitionRepo {
        async fn list(&self, limit: i64, offset: i64) -> r_data_core_core::error::Result<Vec<EntityDefinition>>;
        async fn count(&self) -> r_data_core_core::error::Result<i64>;
        async fn get_by_uuid(&self, uuid: &Uuid) -> r_data_core_core::error::Result<Option<EntityDefinition>>;
        async fn get_by_entity_type(&self, entity_type: &str) -> r_data_core_core::error::Result<Option<EntityDefinition>>;
        async fn create(&self, definition: &EntityDefinition) -> r_data_core_core::error::Result<Uuid>;
        async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> r_data_core_core::error::Result<()>;
        async fn delete(&self, uuid: &Uuid) -> r_data_core_core::error::Result<()>;
        async fn apply_schema(&self, schema_sql: &str) -> r_data_core_core::error::Result<()>;
        async fn update_entity_view_for_entity_definition(&self, entity_definition: &EntityDefinition) -> r_data_core_core::error::Result<()>;
        async fn check_view_exists(&self, view_name: &str) -> r_data_core_core::error::Result<bool>;
        async fn get_view_columns_with_types(&self, view_name: &str) -> r_data_core_core::error::Result<HashMap<String, String>>;
        async fn count_view_records(&self, view_name: &str) -> r_data_core_core::error::Result<i64>;
        async fn cleanup_unused_entity_view(&self) -> r_data_core_core::error::Result<()>;
    }
}

// Test function to create a standard entity definition for tests
fn create_test_entity_definition() -> EntityDefinition {
    let creator_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let field_definitions = vec![
        FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            description: Some("The name field".to_string()),
            field_type: FieldType::String,
            required: true,
            indexed: true,
            filterable: true,
            default_value: None,
            ui_settings: UiSettings::default(),
            constraints: HashMap::default(),
            validation: FieldValidation::default(),
        },
        FieldDefinition {
            name: "age".to_string(),
            display_name: "Age".to_string(),
            description: Some("The age field".to_string()),
            field_type: FieldType::Integer,
            required: false,
            indexed: false,
            filterable: false,
            default_value: None,
            ui_settings: UiSettings::default(),
            constraints: HashMap::default(),
            validation: FieldValidation::default(),
        },
    ];

    // Create a properties map for the schema
    let mut properties = HashMap::new();
    properties.insert(
        "entity_type".to_string(),
        JsonValue::String("TestEntity".to_string()),
    );

    let uuid = Uuid::now_v7();

    EntityDefinition {
        uuid,
        entity_type: "TestEntity".to_string(),
        display_name: "Test Entity".to_string(),
        description: Some("A test entity type".to_string()),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: field_definitions,
        schema: Schema::new(properties),
        created_at: now,
        updated_at: now,
        created_by: creator_id,
        updated_by: None,
        published: false,
        version: 1,
    }
}

#[tokio::test]
async fn test_get_entity_definition_by_uuid_found() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let expected_definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(move |_| Ok(Some(expected_definition.clone())));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.get_entity_definition(&uuid).await?;

    assert_eq!(result.entity_type, "TestEntity");
    assert_eq!(result.display_name, "Test Entity");
    assert_eq!(result.fields.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_get_entity_definition_by_uuid_not_found() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(|_| Ok(None));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.get_entity_definition(&uuid).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::NotFound(msg)) = result {
        assert!(msg.contains(&uuid.to_string()));
    } else {
        panic!("Expected NotFound error");
    }

    Ok(())
}

#[tokio::test]
async fn test_create_entity_definition_success() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let definition = create_test_entity_definition();
    let expected_uuid = definition.uuid;

    // Expect check for duplicate entity type (should return None for success case)
    mock_repo
        .expect_get_by_entity_type()
        .with(eq("TestEntity"))
        .returning(|_| Ok(None));

    mock_repo
        .expect_create()
        .with(predicate::always())
        .returning(move |_| Ok(expected_uuid));

    mock_repo
        .expect_update_entity_view_for_entity_definition()
        .with(predicate::always())
        .returning(|_| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.create_entity_definition(&definition).await?;

    assert_eq!(result, expected_uuid);

    Ok(())
}

#[tokio::test]
async fn test_create_entity_definition_invalid_entity_type() -> Result<()> {
    let mock_repo = MockEntityDefinitionRepo::new();
    let mut definition = create_test_entity_definition();
    definition.entity_type = "123InvalidStart".to_string();

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.create_entity_definition(&definition).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
        assert!(msg.contains("must start with a letter"));
    } else {
        panic!("Expected Validation error");
    }

    Ok(())
}

#[tokio::test]
async fn test_create_entity_definition_reserved_word() -> Result<()> {
    let mock_repo = MockEntityDefinitionRepo::new();
    let mut definition = create_test_entity_definition();
    definition.entity_type = "table".to_string();

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.create_entity_definition(&definition).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
        assert!(msg.contains("reserved word"));
    } else {
        panic!("Expected Validation error");
    }

    Ok(())
}

#[tokio::test]
async fn test_create_entity_definition_duplicate_field_names() -> Result<()> {
    let mock_repo = MockEntityDefinitionRepo::new();
    let mut definition = create_test_entity_definition();

    // Add a duplicate field name (case insensitive)
    definition.fields.push(FieldDefinition {
        name: "Name".to_string(), // Duplicate of "name" (case insensitive)
        display_name: "Another Name".to_string(),
        description: None,
        field_type: FieldType::String,
        required: false,
        indexed: false,
        filterable: false,
        default_value: None,
        ui_settings: UiSettings::default(),
        constraints: HashMap::default(),
        validation: FieldValidation::default(),
    });

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.create_entity_definition(&definition).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
        assert!(msg.contains("Duplicate field name"));
    } else {
        panic!("Expected Validation error");
    }

    Ok(())
}

#[tokio::test]
async fn test_delete_entity_definition_with_records() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(move |_| Ok(Some(definition.clone())));

    mock_repo.expect_check_view_exists().returning(|_| Ok(true));

    mock_repo.expect_count_view_records().returning(|_| Ok(10)); // 10 records exist

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.delete_entity_definition(&uuid).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
        assert!(msg.contains("Cannot delete entity definition that has 10 entities"));
    } else {
        panic!("Expected Validation error");
    }

    Ok(())
}

#[tokio::test]
async fn test_delete_entity_definition_success() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(move |_| Ok(Some(definition.clone())));

    mock_repo.expect_check_view_exists().returning(|_| Ok(true));

    mock_repo.expect_count_view_records().returning(|_| Ok(0)); // No records

    mock_repo
        .expect_delete()
        .withf(move |id| id == &uuid)
        .returning(|_| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.delete_entity_definition(&uuid).await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_apply_schema_specific_uuid() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(move |_| Ok(Some(definition.clone())));

    mock_repo
        .expect_update_entity_view_for_entity_definition()
        .returning(|_| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.apply_schema(Some(&uuid)).await?;

    assert_eq!(result.0, 1); // 1 success
    assert!(result.1.is_empty()); // 0 failures

    Ok(())
}

#[tokio::test]
async fn test_apply_schema_all() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let definitions = vec![
        create_test_entity_definition(),
        create_test_entity_definition(),
        create_test_entity_definition(),
    ];

    mock_repo
        .expect_list()
        .withf(|limit, offset| *limit == 1000 && *offset == 0)
        .returning(move |_, _| Ok(definitions.clone()));

    mock_repo
        .expect_update_entity_view_for_entity_definition()
        .times(3)
        .returning(|_| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.apply_schema(None).await?;

    assert_eq!(result.0, 3); // 3 successes
    assert!(result.1.is_empty()); // 0 failures

    Ok(())
}

#[tokio::test]
async fn test_get_entity_definition_by_entity_type_found() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let entity_type = "TestEntity";
    let expected_definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_entity_type()
        .withf(move |et| et == entity_type)
        .returning(move |_| Ok(Some(expected_definition.clone())));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service
        .get_entity_definition_by_entity_type(entity_type)
        .await?;

    assert_eq!(result.entity_type, "TestEntity");
    assert_eq!(result.display_name, "Test Entity");
    assert_eq!(result.fields.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_get_entity_definition_by_entity_type_not_found() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let entity_type = "NonExistentEntity";

    mock_repo
        .expect_get_by_entity_type()
        .withf(move |et| et == entity_type)
        .returning(|_| Ok(None));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service
        .get_entity_definition_by_entity_type(entity_type)
        .await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::NotFound(msg)) = result {
        assert!(msg.contains(entity_type));
    } else {
        panic!("Expected NotFound error");
    }

    Ok(())
}

#[tokio::test]
async fn test_update_entity_definition_success() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(|_| {
            // Create a fresh definition for each call to avoid ownership issues
            let def = create_test_entity_definition();
            Ok(Some(def))
        });

    mock_repo
        .expect_update()
        .withf(move |id, _| id == &uuid)
        .returning(|_, _| Ok(()));

    mock_repo
        .expect_update_entity_view_for_entity_definition()
        .returning(|_| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.update_entity_definition(&uuid, &definition).await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_update_entity_definition_not_found() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();
    let definition = create_test_entity_definition();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(|_| Ok(None));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.update_entity_definition(&uuid, &definition).await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::NotFound(msg)) = result {
        assert!(msg.contains(&uuid.to_string()));
    } else {
        panic!("Expected NotFound error");
    }

    Ok(())
}

#[tokio::test]
async fn test_update_entity_definition_invalid_entity_type() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();
    let uuid = Uuid::now_v7();

    // Create definitions separately to avoid borrow issues
    create_test_entity_definition();

    // Create an invalid definition
    let mut invalid_definition = create_test_entity_definition();
    invalid_definition.entity_type = "123InvalidStart".to_string();

    mock_repo
        .expect_get_by_uuid()
        .withf(move |id| id == &uuid)
        .returning(|_| {
            let def = create_test_entity_definition();
            Ok(Some(def))
        });

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service
        .update_entity_definition(&uuid, &invalid_definition)
        .await;

    assert!(result.is_err());
    if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
        assert!(msg.contains("must start with a letter"));
    } else {
        panic!("Expected Validation error");
    }

    Ok(())
}

#[tokio::test]
async fn test_cleanup_unused_entity_tables() -> Result<()> {
    let mut mock_repo = MockEntityDefinitionRepo::new();

    mock_repo
        .expect_cleanup_unused_entity_view()
        .returning(|| Ok(()));

    let service = EntityDefinitionService::new_without_cache(Arc::new(mock_repo));
    let result = service.cleanup_unused_entity_tables().await;

    assert!(result.is_ok());

    Ok(())
}
