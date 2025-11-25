#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::*;
use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::{self, eq};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_core::DynamicEntity;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_persistence::DynamicEntityRepositoryTrait;
use crate::entity_definition::EntityDefinitionService;

mock! {
    pub DynamicEntityRepo {}

    #[async_trait]
    impl DynamicEntityRepositoryTrait for DynamicEntityRepo {
        async fn create(&self, entity: &DynamicEntity) -> Result<()>;
        async fn update(&self, entity: &DynamicEntity) -> Result<()>;
        async fn get_by_type(&self, entity_type: &str, uuid: &Uuid, exclusive_fields: Option<Vec<String>>) -> Result<Option<DynamicEntity>>;
        async fn get_all_by_type(&self, entity_type: &str, limit: i64, offset: i64, exclusive_fields: Option<Vec<String>>) -> Result<Vec<DynamicEntity>>;
        async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()>;
        async fn filter_entities(
            &self,
            entity_type: &str,
            limit: i64,
            offset: i64,
            filters: Option<HashMap<serde_json::Value, serde_json::Value>>,
            search: Option<(String, Vec<String>)>,
            sort: Option<(String, String)>,
            fields: Option<Vec<String>>
        ) -> Result<Vec<DynamicEntity>>;
        async fn count_entities(&self, entity_type: &str) -> Result<i64>;
    }
}

// Create a mock for the entity definition repository trait
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

fn create_test_entity_definition() -> EntityDefinition {
    use r_data_core_core::entity_definition::schema::Schema;
    use r_data_core_core::field::types::FieldType;
    use r_data_core_core::field::FieldDefinition;
    use time::OffsetDateTime;

    EntityDefinition {
        uuid: Uuid::nil(),
        entity_type: "test_entity".to_string(),
        display_name: "Test Entity".to_string(),
        description: Some("Test entity for unit tests".to_string()),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: Some(Uuid::nil()),
        version: 1,
        allow_children: false,
        icon: None,
        group_name: None,
        schema: Schema::default(),
        fields: vec![
            FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: Some("Name field".to_string()),
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
                description: Some("Age field".to_string()),
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
        published: true,
    }
}

fn create_test_entity() -> DynamicEntity {
    let entity_def = create_test_entity_definition();
    let mut field_data = HashMap::new();
    field_data.insert("name".to_string(), json!("Test Entity"));
    field_data.insert("age".to_string(), json!(30));
    field_data.insert("uuid".to_string(), json!(Uuid::nil().to_string()));

    DynamicEntity {
        entity_type: "test_entity".to_string(),
        field_data,
        definition: Arc::new(entity_def),
    }
}

#[tokio::test]
async fn test_list_entities() -> Result<()> {
    let mut repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();

    let entity_type = "test_entity";
    let limit = 10;
    let offset = 0;

    // Setup mock entity definition repository
    class_repo
        .expect_get_by_entity_type()
        .with(predicate::eq(entity_type))
        .returning(|_| Ok(Some(create_test_entity_definition())));

    // Setup mock repository response
    repo.expect_get_all_by_type()
        .with(
            predicate::eq(entity_type),
            predicate::eq(limit),
            predicate::eq(offset),
            predicate::eq(None),
        )
        .returning(|_, _, _, _| Ok(vec![create_test_entity()]));

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    let entities = service
        .list_entities(entity_type, limit, offset, None)
        .await?;

    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].entity_type, entity_type);

    Ok(())
}

#[tokio::test]
async fn test_get_entity_by_uuid() -> Result<()> {
    let mut repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();

    let entity_type = "test_entity";
    let uuid = Uuid::nil();

    // Setup mock entity definition repository
    class_repo
        .expect_get_by_entity_type()
        .with(predicate::eq(entity_type))
        .returning(|_| Ok(Some(create_test_entity_definition())));

    // Setup mock repository response
    repo.expect_get_by_type()
        .with(
            predicate::eq(entity_type),
            predicate::eq(uuid.clone()),
            predicate::eq(None),
        )
        .returning(|_, _, _| Ok(Some(create_test_entity())));

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    let entity = service.get_entity_by_uuid(entity_type, &uuid, None).await?;

    assert!(entity.is_some());
    let entity = entity.unwrap();
    assert_eq!(entity.entity_type, entity_type);

    Ok(())
}

#[tokio::test]
async fn test_create_entity() -> Result<()> {
    let mut repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();

    let entity = create_test_entity();

    // Setup mock entity definition repository to return a published entity definition
    class_repo
        .expect_get_by_entity_type()
        .with(predicate::eq("test_entity"))
        .returning(|_| Ok(Some(create_test_entity_definition())));

    // Setup mock repository response
    repo.expect_create()
        .with(predicate::function(|e: &DynamicEntity| {
            e.entity_type == "test_entity"
        }))
        .returning(|_| Ok(()));

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    let result = service.create_entity(&entity).await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_create_entity_missing_required_field() -> Result<()> {
    let repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();

    // Setup mock entity definition repository
    class_repo
        .expect_get_by_entity_type()
        .with(predicate::eq("test_entity"))
        .returning(move |_| Ok(Some(create_test_entity_definition())));

    // Create a new entity with only age field (missing both uuid and required "name" field)
    // Without the uuid field, this will be treated as a create operation
    let entity = DynamicEntity {
        entity_type: "test_entity".to_string(),
        field_data: {
            let mut fields = HashMap::new();
            // NOT adding uuid field - this is important to test create validation logic
            fields.insert("age".to_string(), json!(30));
            // Explicitly NOT adding the required "name" field
            fields
        },
        definition: Arc::new(create_test_entity_definition()),
    };

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    // Try to create the entity, should fail because of missing required field
    let result = service.create_entity(&entity).await;

    // Check that we got a validation error
    assert!(result.is_err());
    match result {
        Err(r_data_core_core::error::Error::Validation(msg)) => {
            assert!(msg.contains("Required field 'name' is missing"));
        }
        _ => panic!("Expected validation error, got: {:?}", result),
    }

    Ok(())
}

#[test]
fn test_get_entity_by_uuid_sync() {
    let mut repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();
    let entity_type = "test_entity";

    // Create a UUID that lives for the entire test function
    let uuid = Uuid::nil();

    repo.expect_get_by_type()
        .with(eq(entity_type), eq(uuid.clone()), eq(None))
        .returning(|_, _, _| Ok(Some(create_test_entity())));

    class_repo
        .expect_get_by_entity_type()
        .with(eq(entity_type))
        .returning(|_| Ok(Some(create_test_entity_definition())));

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(service.get_entity_by_uuid(entity_type, &uuid, None));
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_entity_by_type() -> Result<()> {
    let mut repo = MockDynamicEntityRepo::new();
    let mut class_repo = MockEntityDefinitionRepo::new();

    let entity_type = "test_entity";

    // Setup mock entity definition repository
    class_repo
        .expect_get_by_entity_type()
        .with(predicate::eq(entity_type))
        .returning(|_| Ok(Some(create_test_entity_definition())));

    // Setup mock repository response for the entity type
    repo.expect_get_all_by_type()
        .with(
            predicate::eq(entity_type),
            predicate::always(),
            predicate::always(),
            predicate::eq(None),
        )
        .returning(|_, _, _, _| Ok(vec![create_test_entity()]));

    // Create service with proper mocks
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

    let entities = service.list_entities(entity_type, 10, 0, None).await?;

    assert!(!entities.is_empty());
    assert_eq!(entities[0].entity_type, entity_type);

    Ok(())
}

