use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use mockall::predicate;
use serde_json::{json, Value};
use uuid::Uuid;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::DynamicEntityRepositoryTrait;

// Create a struct to represent DynamicFields since we can't use the trait directly
#[derive(Default)]
struct TestDynamicFields(HashMap<String, Value>);

impl TestDynamicFields {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn insert(&mut self, key: String, value: Value) {
        self.0.insert(key, value);
    }

    fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

// Create a mock for DynamicEntityRepositoryTrait
mockall::mock! {
    pub DynamicEntityRepositoryTrait {}

    #[async_trait]
    impl DynamicEntityRepositoryTrait for DynamicEntityRepositoryTrait {
        async fn get_all_by_type(&self, entity_type: &str, limit: i64, offset: i64, exclusive_fields: Option<Vec<String>>) -> Result<Vec<DynamicEntity>>;
        async fn get_by_type(&self, entity_type: &str, uuid: &Uuid, exclusive_fields: Option<Vec<String>>) -> Result<Option<DynamicEntity>>;
        async fn create(&self, entity: &DynamicEntity) -> Result<()>;
        async fn update(&self, entity: &DynamicEntity) -> Result<()>;
        async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()>;
        async fn filter_entities(
            &self,
            entity_type: &str,
            limit: i64,
            offset: i64,
            filters: Option<HashMap<String, Value>>,
            search: Option<(String, Vec<String>)>,
            sort: Option<(String, String)>,
            fields: Option<Vec<String>>
        ) -> Result<Vec<DynamicEntity>>;
        async fn count_entities(&self, entity_type: &str) -> Result<i64>;
    }
}

// Create a mockable EntityDefinitionService
#[derive(Clone)]
struct MockEntityDefinitionService {
    entity_type_exists: bool,
    entity_type_published: bool,
}

impl MockEntityDefinitionService {
    fn new(entity_type_exists: bool, entity_type_published: bool) -> Self {
        Self {
            entity_type_exists,
            entity_type_published,
        }
    }

    async fn get_entity_definition_by_entity_type(
        &self,
        entity_type: &str,
    ) -> Result<EntityDefinition> {
        if !self.entity_type_exists {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Class definition for entity type '{}' not found",
                entity_type
            )));
        }

        let mut definition = EntityDefinition::default();
        definition.entity_type = entity_type.to_string();
        definition.published = self.entity_type_published;

        // Add fields to the definition
        let required_field = FieldDefinition {
            name: "required_field".to_string(),
            field_type: FieldType::Text,
            required: true,
            display_name: "Required Field".to_string(),
            description: None,
            filterable: false,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let optional_field = FieldDefinition {
            name: "optional_field".to_string(),
            field_type: FieldType::Text,
            required: false,
            display_name: "Optional Field".to_string(),
            description: None,
            filterable: false,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let string_field = FieldDefinition {
            name: "email_field".to_string(),
            field_type: FieldType::Text,
            required: false,
            display_name: "Email Field".to_string(),
            description: None,
            filterable: false,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let number_field = FieldDefinition {
            name: "score".to_string(),
            field_type: FieldType::Text,
            required: false,
            display_name: "Score".to_string(),
            description: None,
            filterable: false,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        let enum_field = FieldDefinition {
            name: "status".to_string(),
            field_type: FieldType::Text,
            required: false,
            display_name: "Status".to_string(),
            description: None,
            filterable: false,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        };

        definition.fields = vec![
            required_field,
            optional_field,
            string_field,
            number_field,
            enum_field,
        ];

        Ok(definition)
    }

    async fn _get_entity_definition(&self, _uuid: &Uuid) -> Result<EntityDefinition> {
        if !self.entity_type_exists {
            return Err(r_data_core_core::error::Error::NotFound(
                "Class definition not found".to_string(),
            ));
        }

        let mut definition = EntityDefinition::default();
        definition.entity_type = "test_entity".to_string();
        definition.published = self.entity_type_published;

        Ok(definition)
    }
}

fn create_test_entity(entity_type: &str, with_required_field: bool) -> DynamicEntity {
    let uuid = Uuid::nil();
    let definition = Arc::new(EntityDefinition::default());

    // Create the entity with a valid definition
    let mut entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data: HashMap::new(),
        definition,
    };

    entity
        .field_data
        .insert("uuid".to_string(), json!(uuid.to_string()));

    if with_required_field {
        entity
            .field_data
            .insert("required_field".to_string(), json!("value"));
    }

    entity
}

// Create a mocked service for testing
struct TestService {
    repository: MockDynamicEntityRepositoryTrait,
    class_service: MockEntityDefinitionService,
}

impl TestService {
    fn new(entity_type_exists: bool, entity_type_published: bool) -> Self {
        Self {
            repository: MockDynamicEntityRepositoryTrait::new(),
            class_service: MockEntityDefinitionService::new(
                entity_type_exists,
                entity_type_published,
            ),
        }
    }

    async fn list_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // First check if the entity type exists and is published
        let entity_def = self
            .class_service
            .get_entity_definition_by_entity_type(entity_type)
            .await?;

        if !entity_def.published {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity type '{}' not found or not published",
                entity_type
            )));
        }

        self.repository
            .get_all_by_type(entity_type, limit, offset, exclusive_fields)
            .await
    }

    async fn create_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Check if the entity type is published
        let entity_def = self
            .class_service
            .get_entity_definition_by_entity_type(&entity.entity_type)
            .await?;

        if !entity_def.published {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity type '{}' not found or not published",
                entity.entity_type
            )));
        }

        // Very basic validation - check for required fields
        for field in &entity_def.fields {
            if field.required && !entity.field_data.contains_key(&field.name) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Required field '{}' is missing",
                    field.name
                )));
            }
        }

        self.repository.create(entity).await
    }
}

#[tokio::test]
async fn test_list_entities_success() -> Result<()> {
    // Arrange
    let mut test_service = TestService::new(true, true);

    // Set up expectations directly
    test_service
        .repository
        .expect_get_all_by_type()
        .with(
            predicate::eq("test_entity"),
            predicate::eq(10),
            predicate::eq(0),
            predicate::always(),
        )
        .returning(|_, _, _, _| Ok(vec![create_test_entity("test_entity", true)]));

    // Act
    let entities = test_service
        .list_entities("test_entity", 10, 0, None)
        .await?;

    // Assert
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].entity_type, "test_entity");

    Ok(())
}

#[tokio::test]
async fn test_list_entities_nonexistent_type() {
    // Arrange
    let test_service = TestService::new(false, false);

    // Act
    let result = test_service
        .list_entities("nonexistent_type", 10, 0, None)
        .await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(r_data_core_core::error::Error::NotFound(_)) => (),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_list_entities_unpublished_type() {
    // Arrange
    let test_service = TestService::new(true, false);

    // Act
    let result = test_service.list_entities("test_entity", 10, 0, None).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(r_data_core_core::error::Error::NotFound(_)) => (),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_create_entity_success() -> Result<()> {
    // Arrange
    let mut test_service = TestService::new(true, true);

    // Set up expectations directly
    test_service
        .repository
        .expect_create()
        .with(predicate::always())
        .returning(|_| Ok(()));

    let entity = create_test_entity("test_entity", true);

    // Act
    let result = test_service.create_entity(&entity).await;

    // Assert
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_create_entity_missing_required_field() {
    // Arrange
    let test_service = TestService::new(true, true);

    let entity = create_test_entity("test_entity", false);

    // Act
    let result = test_service.create_entity(&entity).await;

    // Assert
    assert!(result.is_err());
    match result {
        Err(r_data_core_core::error::Error::Validation(_)) => (),
        _ => panic!("Expected Validation error"),
    }
}
