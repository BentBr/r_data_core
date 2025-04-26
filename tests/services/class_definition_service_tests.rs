use async_trait::async_trait;
use mockall::predicate::eq;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core::{
    entity::class::definition::ClassDefinition,
    entity::class::repository_trait::ClassDefinitionRepositoryTrait,
    entity::class::schema::Schema,
    entity::field::definition::FieldDefinition,
    entity::field::types::FieldType,
    error::{Error, Result},
    services::ClassDefinitionService,
};

// Create a mock for ClassDefinitionRepositoryTrait
mockall::mock! {
    pub ClassDefRepository {}

    #[async_trait]
    impl ClassDefinitionRepositoryTrait for ClassDefRepository {
        async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>>;
        async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<ClassDefinition>>;
        async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>>;
        async fn create(&self, definition: &ClassDefinition) -> Result<Uuid>;
        async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()>;
        async fn delete(&self, uuid: &Uuid) -> Result<()>;
        async fn apply_schema(&self, schema_sql: &str) -> Result<()>;
        async fn update_entity_view_for_class_definition(&self, class_definition: &ClassDefinition) -> Result<()>;
        async fn check_view_exists(&self, view_name: &str) -> Result<bool>;
        async fn get_view_columns_with_types(&self, view_name: &str) -> Result<HashMap<String, String>>;
        async fn count_view_records(&self, view_name: &str) -> Result<i64>;
        async fn cleanup_unused_entity_view(&self) -> Result<()>;
    }
}

// Helper function to create a test class definition
fn create_test_class_definition() -> ClassDefinition {
    ClassDefinition {
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
                default_value: None,
                validation: Default::default(),
                ui_settings: Default::default(),
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
                default_value: None,
                validation: Default::default(),
                ui_settings: Default::default(),
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

// Helper function to create an invalid class definition for testing validation
fn create_invalid_class_definition() -> ClassDefinition {
    let mut def = create_test_class_definition();
    def.entity_type = "Invalid-Type".to_string(); // Contains invalid characters
    def
}

// Helper function to create a class definition with duplicate field names
fn create_class_definition_with_duplicate_fields() -> ClassDefinition {
    let mut def = create_test_class_definition();
    def.fields.push(FieldDefinition {
        name: "name".to_string(), // Duplicate name
        display_name: "Name 2".to_string(),
        description: Some("Duplicate field".to_string()),
        field_type: FieldType::String,
        required: false,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: Default::default(),
        ui_settings: Default::default(),
        constraints: HashMap::new(),
    });
    def
}

#[tokio::test]
async fn test_list_class_definitions() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let expected_definitions = vec![
        create_test_class_definition(),
        create_test_class_definition(),
    ];

    mock_repo
        .expect_list()
        .with(eq(10), eq(0))
        .return_once(move |_, _| Ok(expected_definitions));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.list_class_definitions(10, 0).await?;

    // Assert
    assert_eq!(result.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_get_class_definition_by_uuid_found() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let test_uuid = Uuid::now_v7();
    let expected_definition = create_test_class_definition();

    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(move |_| Ok(Some(expected_definition)));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.get_class_definition(&test_uuid).await?;

    // Assert
    assert_eq!(result.entity_type, "test_entity");

    Ok(())
}

#[tokio::test]
async fn test_get_class_definition_by_uuid_not_found() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let test_uuid = Uuid::now_v7();

    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(|_| Ok(None));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.get_class_definition(&test_uuid).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::NotFound(_)) => {}
        _ => panic!("Expected not found error"),
    }

    Ok(())
}

#[tokio::test]
async fn test_create_class_definition_success() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let definition = create_test_class_definition();
    let expected_uuid = definition.uuid;

    // No existing definition with the same entity type
    mock_repo
        .expect_get_by_entity_type()
        .with(eq("test_entity"))
        .return_once(|_| Ok(None));

    // Create should succeed
    mock_repo
        .expect_create()
        .return_once(move |_| Ok(expected_uuid));

    // Update view should succeed
    mock_repo
        .expect_update_entity_view_for_class_definition()
        .return_once(|_| Ok(()));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.create_class_definition(&definition).await?;

    // Assert
    assert_eq!(result, expected_uuid);

    Ok(())
}

#[tokio::test]
async fn test_create_class_definition_duplicate_entity_type() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let definition = create_test_class_definition();
    let existing_definition = create_test_class_definition();

    // First mock get_by_entity_type to check for existing entity type
    mock_repo
        .expect_get_by_entity_type()
        .with(eq("test_entity"))
        .return_once(move |_| Ok(Some(existing_definition)));

    // The service should now return an error before reaching the create call,
    // so we no longer need to mock the create method

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.create_class_definition(&definition).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::ClassAlreadyExists(e)) => {
            assert!(
                e.contains("already exists"),
                "Error should mention duplicate entity type: {}",
                e
            );
        }
        _ => panic!(
            "Expected validation error for duplicate entity type, got: {:?}",
            result
        ),
    }

    Ok(())
}

#[tokio::test]
async fn test_create_class_definition_invalid_entity_type() -> Result<()> {
    // Arrange
    let mock_repo = MockClassDefRepository::new();
    let invalid_definition = create_invalid_class_definition();

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.create_class_definition(&invalid_definition).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::Validation(e)) => {
            assert!(
                e.contains("must start with a letter") && e.contains("Entity type"),
                "Error should mention invalid entity type format: {}",
                e
            );
        }
        _ => panic!("Expected validation error for invalid entity type"),
    }

    Ok(())
}

#[tokio::test]
async fn test_create_class_definition_duplicate_field_names() -> Result<()> {
    // Arrange
    let mock_repo = MockClassDefRepository::new();
    let definition_with_duplicates = create_class_definition_with_duplicate_fields();

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service
        .create_class_definition(&definition_with_duplicates)
        .await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::Validation(e)) => {
            assert!(
                e.contains("Duplicate"),
                "Error should mention duplicate field names"
            );
        }
        _ => panic!("Expected validation error for duplicate field names"),
    }

    Ok(())
}

#[tokio::test]
async fn test_update_class_definition_success() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let definition = create_test_class_definition();
    let test_uuid = definition.uuid;
    let definition_clone = definition.clone();

    // Return the existing definition for validation
    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(move |_| Ok(Some(definition_clone)));

    // No other definition with the same entity type
    mock_repo
        .expect_get_by_entity_type()
        .with(eq("test_entity"))
        .return_once(|_| Ok(None));

    // Update should succeed
    mock_repo.expect_update().return_once(|_, _| Ok(()));

    // Update view should succeed
    mock_repo
        .expect_update_entity_view_for_class_definition()
        .return_once(|_| Ok(()));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service
        .update_class_definition(&test_uuid, &definition)
        .await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_update_class_definition_not_found() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let definition = create_test_class_definition();
    let test_uuid = definition.uuid;

    // No existing definition found
    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(|_| Ok(None));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service
        .update_class_definition(&test_uuid, &definition)
        .await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::NotFound(_)) => {}
        _ => panic!("Expected not found error"),
    }

    Ok(())
}

#[tokio::test]
async fn test_delete_class_definition_success() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let test_uuid = Uuid::now_v7();
    let test_definition = create_test_class_definition();

    // Set up expectations for a class definition existing
    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(move |_| Ok(Some(test_definition)));

    // Expect a call to check if the view exists
    mock_repo
        .expect_check_view_exists()
        .return_once(|_| Ok(true));

    // No records exist for this class
    mock_repo.expect_count_view_records().return_once(|_| Ok(0));

    // Delete should succeed
    mock_repo.expect_delete().return_once(|_| Ok(()));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.delete_class_definition(&test_uuid).await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_delete_class_definition_with_records() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let test_uuid = Uuid::now_v7();
    let test_definition = create_test_class_definition();

    // Set up expectations for a class definition existing
    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(move |_| Ok(Some(test_definition)));

    // Expect a call to check if the view exists
    mock_repo
        .expect_check_view_exists()
        .return_once(|_| Ok(true));

    // Records exist for this class
    mock_repo.expect_count_view_records().return_once(|_| Ok(5));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.delete_class_definition(&test_uuid).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::Validation(e)) => {
            assert!(
                e.contains("entities"),
                "Error should mention existing records"
            );
        }
        _ => panic!("Expected validation error for existing records"),
    }

    Ok(())
}

#[tokio::test]
async fn test_delete_class_definition_not_found() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();
    let test_uuid = Uuid::now_v7();

    // No existing definition found
    mock_repo
        .expect_get_by_uuid()
        .with(eq(test_uuid.clone()))
        .return_once(|_| Ok(None));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.delete_class_definition(&test_uuid).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(Error::NotFound(_)) => {}
        _ => panic!("Expected not found error"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cleanup_unused_entity_tables() -> Result<()> {
    // Arrange
    let mut mock_repo = MockClassDefRepository::new();

    // Cleanup should succeed
    mock_repo
        .expect_cleanup_unused_entity_view()
        .return_once(|| Ok(()));

    let service = ClassDefinitionService::new(Arc::new(mock_repo));

    // Act
    let result = service.cleanup_unused_entity_tables().await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}
