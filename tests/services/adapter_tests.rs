#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use mockall::predicate::eq;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::{
    entity_definition::definition::EntityDefinition,
    entity_definition::repository_trait::EntityDefinitionRepositoryTrait,
    entity_definition::schema::Schema, field::definition::FieldDefinition, field::types::FieldType,
};

// Create a trait-based mock for testing
mockall::mock! {
    pub EntityDefRepository { }

    #[async_trait]
    impl r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait for EntityDefRepository {
        async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>>;
        async fn count(&self) -> Result<i64>;
        async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<EntityDefinition>>;
        async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>>;
        async fn create(&self, definition: &EntityDefinition) -> Result<Uuid>;
        async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()>;
        async fn delete(&self, uuid: &Uuid) -> Result<()>;
        async fn apply_schema(&self, schema_sql: &str) -> Result<()>;
        async fn update_entity_view_for_entity_definition(&self, entity_definition: &EntityDefinition) -> Result<()>;
        async fn check_view_exists(&self, view_name: &str) -> Result<bool>;
        async fn get_view_columns_with_types(&self, view_name: &str) -> Result<HashMap<String, String>>;
        async fn count_view_records(&self, view_name: &str) -> Result<i64>;
        async fn cleanup_unused_entity_view(&self) -> Result<()>;
    }
}

// Helper function to create a test entity definition
fn create_test_entity_definition() -> EntityDefinition {
    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: "test_entity".to_string(),
        display_name: "Test Entity".to_string(),
        description: Some("Test Description".to_string()),
        group_name: Some("Test Group".to_string()),
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: Some("Person's name".to_string()),
                field_type: FieldType::String,
                required: true,
                indexed: true,
                filterable: true,
                unique: false,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "age".to_string(),
                display_name: "Age".to_string(),
                description: Some("Person's age".to_string()),
                field_type: FieldType::Integer,
                required: false,
                indexed: false,
                filterable: true,
                unique: false,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: Schema::default(),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::now_v7(),
        updated_by: Some(Uuid::now_v7()),
        published: false,
        version: 1,
    }
}

#[tokio::test]
async fn test_trait_list_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let expected_definitions = vec![create_test_entity_definition()];

    // Setup expectations
    mock.expect_list()
        .with(eq(10), eq(0))
        .returning(move |_, _| Ok(expected_definitions.clone()));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.list(10, 0).await?;

    // Assert
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].entity_type, "test_entity");

    Ok(())
}

#[tokio::test]
async fn test_trait_get_by_uuid_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let test_uuid = Uuid::now_v7();
    let test_definition = create_test_entity_definition();

    // Setup expectations - use a matcher function that captures by value
    mock.expect_get_by_uuid()
        .withf(move |uuid: &Uuid| *uuid == test_uuid)
        .return_once(move |_| Ok(Some(test_definition)));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.get_by_uuid(&test_uuid).await?;

    // Assert
    assert!(result.is_some());
    let def = result.unwrap();
    assert_eq!(def.entity_type, "test_entity");

    Ok(())
}

#[tokio::test]
async fn test_trait_create_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let test_definition = create_test_entity_definition();
    let expected_uuid = test_definition.uuid;

    // Setup expectations
    mock.expect_create()
        .withf(|def: &EntityDefinition| def.entity_type == "test_entity")
        .return_once(move |_| Ok(expected_uuid));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.create(&test_definition).await?;

    // Assert
    assert_eq!(result, expected_uuid);

    Ok(())
}

#[tokio::test]
async fn test_trait_update_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let test_definition = create_test_entity_definition();
    let test_uuid = test_definition.uuid;

    // Setup expectations
    mock.expect_update()
        .withf(move |uuid: &Uuid, def: &EntityDefinition| {
            *uuid == test_uuid && def.entity_type == "test_entity"
        })
        .return_once(|_, _| Ok(()));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.update(&test_uuid, &test_definition).await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_trait_delete_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let test_uuid = Uuid::now_v7();

    // Setup expectations - use a matcher function that captures by value
    mock.expect_delete()
        .withf(move |uuid: &Uuid| *uuid == test_uuid)
        .return_once(|_| Ok(()));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.delete(&test_uuid).await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_trait_check_view_exists_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let view_name = "test_view";

    // Setup expectations
    mock.expect_check_view_exists()
        .with(eq(view_name))
        .return_once(|_| Ok(true));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.check_view_exists(view_name).await?;

    // Assert
    assert!(result);

    Ok(())
}

#[tokio::test]
async fn test_trait_apply_schema_delegates_correctly() -> Result<()> {
    // Arrange
    let mut mock = MockEntityDefRepository::new();
    let schema_sql = "CREATE TABLE test (id UUID PRIMARY KEY);";

    // Setup expectations
    mock.expect_apply_schema()
        .with(eq(schema_sql))
        .return_once(|_| Ok(()));

    // Use the mock as a trait object
    let repo: Arc<dyn EntityDefinitionRepositoryTrait> = Arc::new(mock);

    // Act
    let result = repo.apply_schema(schema_sql).await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}
