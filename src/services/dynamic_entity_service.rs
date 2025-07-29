use std::collections::HashMap;
use std::sync::Arc;

use log::debug;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use crate::entity::field::types::FieldType;
use crate::error::{Error, Result};
use crate::services::ClassDefinitionService;

/// Service for managing dynamic entities with validation based on class definitions
#[derive(Clone)]
pub struct DynamicEntityService {
    repository: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
    class_definition_service: Arc<ClassDefinitionService>,
}

impl DynamicEntityService {
    /// Create a new DynamicEntityService with the provided repository and class definition service
    pub fn new(
        repository: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
        class_definition_service: Arc<ClassDefinitionService>,
    ) -> Self {
        DynamicEntityService {
            repository,
            class_definition_service,
        }
    }

    /// Get the underlying repository - helper for debugging
    pub fn get_repository(&self) -> &Arc<dyn DynamicEntityRepositoryTrait + Send + Sync> {
        &self.repository
    }

    // Check if the entity type exists and is published - common check for all operations
    async fn check_entity_type_exists_and_published(
        &self,
        entity_type: &str,
    ) -> Result<ClassDefinition> {
        let class_definition = self
            .class_definition_service
            .get_class_definition_by_entity_type(entity_type)
            .await?;

        if !class_definition.published {
            return Err(Error::NotFound(format!(
                "Entity type '{}' not found or not published",
                entity_type
            )));
        }

        Ok(class_definition)
    }

    /// List entities with pagination
    pub async fn list_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository
            .get_all_by_type(entity_type, limit, offset, exclusive_fields)
            .await
    }

    /// Count entities of a specific type
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository.count_entities(entity_type).await
    }

    /// Get an entity by UUID
    pub async fn get_entity_by_uuid(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository
            .get_by_type(entity_type, uuid, exclusive_fields)
            .await
    }

    /// Create a new entity with validation
    pub async fn create_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against class definition
        self.validate_entity(entity)?;

        self.repository.create(entity).await
    }

    /// Update an existing entity with validation
    pub async fn update_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against class definition
        self.validate_entity(entity)?;

        self.repository.update(entity).await
    }

    /// Delete an entity
    pub async fn delete_entity(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository.delete_by_type(entity_type, uuid).await
    }

    /// Validate an entity against its class definition
    fn validate_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Collect all validation errors instead of returning on first error
        let mut validation_errors = Vec::new();

        // Check for unknown fields - fields in the data that are not defined in the class definition
        let unknown_fields = self.check_unknown_fields(entity);
        if !unknown_fields.is_empty() {
            validation_errors.push(format!(
                "Unknown fields found: {}. Only fields defined in the class definition are allowed.",
                unknown_fields.join(", ")
            ));
        }

        // For update operations, we only need to validate the fields that are being submitted
        // For create operations, check all required fields
        let is_update = self.is_update_operation(entity);
        debug!("Validation - is update operation: {}", is_update);

        if !is_update {
            // This is a create operation, so check all required fields
            self.check_required_fields(entity, &mut validation_errors);
        }

        // Validate field values against their types and constraints (only for fields that are present)
        self.validate_field_values(entity, &mut validation_errors);

        // If we've collected any errors, return them all as one validation error
        if !validation_errors.is_empty() {
            return Err(Error::Validation(format!(
                "Validation failed with the following errors: {}",
                validation_errors.join("; ")
            )));
        }

        Ok(())
    }

    // Check if this is an update operation based on presence of UUID
    fn is_update_operation(&self, entity: &DynamicEntity) -> bool {
        entity.field_data.contains_key("uuid") && entity.field_data.get("uuid").is_some()
    }

    // Check for unknown fields
    fn check_unknown_fields(&self, entity: &DynamicEntity) -> Vec<String> {
        let reserved_fields = [
            "uuid",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        let mut unknown_fields = Vec::new();
        for field_name in entity.field_data.keys() {
            // Skip system/reserved fields
            if reserved_fields.contains(&field_name.as_str()) {
                continue;
            }

            // Check if this field exists in the class definition
            if !entity
                .definition
                .fields
                .iter()
                .any(|f| f.name == *field_name)
            {
                unknown_fields.push(field_name.clone());
            }
        }

        unknown_fields
    }

    // Check required fields
    fn check_required_fields(&self, entity: &DynamicEntity, validation_errors: &mut Vec<String>) {
        for field in &entity.definition.fields {
            if field.required && !entity.field_data.contains_key(&field.name) {
                validation_errors.push(format!("Required field '{}' is missing", field.name));
            }
        }
    }

    // Validate field values
    fn validate_field_values(&self, entity: &DynamicEntity, validation_errors: &mut Vec<String>) {
        for field in &entity.definition.fields {
            if let Some(value) = entity.field_data.get(&field.name) {
                if let Err(e) = field.validate_value(value) {
                    validation_errors
                        .push(format!("Field '{}' validation error: {}", field.name, e));
                }
            }
        }
    }

    /// Filter entities based on field values
    pub async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository
            .filter_entities(entity_type, limit, offset, filters, search, sort, fields)
            .await
    }

    /// Validate an entity against its class definition - exported for testing
    #[cfg(test)]
    pub fn validate_entity_for_test(&self, entity: &DynamicEntity) -> Result<()> {
        self.validate_entity(entity)
    }

    /// List entities with advanced filtering options
    pub async fn list_entities_with_filters(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        fields: Option<Vec<String>>,
        sort_by: Option<String>,
        sort_direction: Option<String>,
        filter: Option<serde_json::Value>,
        search_query: Option<String>,
    ) -> Result<(Vec<DynamicEntity>, i64)> {
        // Verify the entity type exists and is published
        let class_def = self.get_class_definition_for_query(entity_type).await?;

        // Count entities first for pagination
        let total = self.repository.count_entities(entity_type).await?;

        // Build filter conditions from the structured filter
        let mut filter_conditions = HashMap::new();

        if let Some(filter_value) = filter {
            if let Some(obj) = filter_value.as_object() {
                for (key, value) in obj {
                    filter_conditions.insert(key.clone(), value.clone());
                }
            }
        }

        // Add search query if provided
        let search_fields = if let Some(query) = search_query {
            // Get text/string fields from class definition for searching
            let searchable_fields: Vec<String> = class_def
                .fields
                .iter()
                .filter(|field| {
                    matches!(
                        field.field_type,
                        FieldType::String | FieldType::Text | FieldType::Wysiwyg
                    )
                })
                .map(|field| field.name.clone())
                .collect();

            // Return the query and fields to search in
            if !searchable_fields.is_empty() {
                Some((query, searchable_fields))
            } else {
                None
            }
        } else {
            None
        };

        // Build sort information
        let sort_info = if let Some(field) = sort_by {
            let direction = sort_direction.unwrap_or_else(|| "ASC".to_string());
            Some((field, direction))
        } else {
            // Default sort by created_at descending if not specified
            Some(("created_at".to_string(), "DESC".to_string()))
        };

        // Fetch the entities
        let entities = self
            .repository
            .filter_entities(
                entity_type,
                limit,
                offset,
                Some(filter_conditions),
                search_fields,
                sort_info,
                fields,
            )
            .await?;

        Ok((entities, total))
    }

    /// Helper method to get class definition for query operations
    async fn get_class_definition_for_query(&self, entity_type: &str) -> Result<ClassDefinition> {
        // Look up the class definition
        let class_def = match self
            .class_definition_service
            .get_class_definition_by_entity_type(entity_type)
            .await
        {
            Ok(class_def) => class_def,
            Err(Error::NotFound(_)) => {
                return Err(Error::NotFound(format!(
                    "Entity type '{}' not found",
                    entity_type
                )));
            }
            Err(e) => return Err(e),
        };

        // Ensure the class is published
        if !class_def.published {
            return Err(Error::ValidationFailed(format!(
                "Entity type '{}' is not published",
                entity_type
            )));
        }

        Ok(class_def)
    }

    async fn get_entities_with_filters(
        &self,
        entity_type: &str,
        filters: Option<HashMap<String, JsonValue>>,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // If no filters, use the standard method
        if filters.is_none() {
            return self
                .repository
                .get_all_by_type(entity_type, limit, offset, exclusive_fields)
                .await;
        }

        // Validate entity type
        let _ = self.get_class_definition_for_query(entity_type).await?;

        // Use the new filter_entities method with the structured parameters
        self.repository
            .filter_entities(
                entity_type,
                limit,
                offset,
                filters,
                None, // no search
                None, // no sort
                exclusive_fields,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::{self, eq};
    use serde_json::json;

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
                filters: Option<HashMap<String, JsonValue>>,
                search: Option<(String, Vec<String>)>,
                sort: Option<(String, String)>,
                fields: Option<Vec<String>>
            ) -> Result<Vec<DynamicEntity>>;
            async fn count_entities(&self, entity_type: &str) -> Result<i64>;
        }
    }

    // Create a mock for the class definition repository trait
    mock! {
        pub ClassDefinitionRepo {}

        #[async_trait]
        impl crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait for ClassDefinitionRepo {
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

    fn create_test_class_definition() -> ClassDefinition {
        use crate::entity::class::schema::Schema;
        use crate::entity::field::types::FieldType;
        use crate::entity::field::FieldDefinition;
        use time::OffsetDateTime;

        ClassDefinition {
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
        let class_def = create_test_class_definition();
        let mut field_data = HashMap::new();
        field_data.insert("name".to_string(), json!("Test Entity"));
        field_data.insert("age".to_string(), json!(30));
        field_data.insert("uuid".to_string(), json!(Uuid::nil().to_string()));

        DynamicEntity {
            entity_type: "test_entity".to_string(),
            field_data,
            definition: Arc::new(class_def),
        }
    }

    #[tokio::test]
    async fn test_list_entities() -> Result<()> {
        let mut repo = MockDynamicEntityRepo::new();
        let mut class_repo = MockClassDefinitionRepo::new();

        let entity_type = "test_entity";
        let limit = 10;
        let offset = 0;

        // Setup mock class definition repository
        class_repo
            .expect_get_by_entity_type()
            .with(predicate::eq(entity_type))
            .returning(|_| Ok(Some(create_test_class_definition())));

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
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
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
        let mut class_repo = MockClassDefinitionRepo::new();

        let entity_type = "test_entity";
        let uuid = Uuid::nil();

        // Setup mock class definition repository
        class_repo
            .expect_get_by_entity_type()
            .with(predicate::eq(entity_type))
            .returning(|_| Ok(Some(create_test_class_definition())));

        // Setup mock repository response
        repo.expect_get_by_type()
            .with(
                predicate::eq(entity_type),
                predicate::eq(uuid.clone()),
                predicate::eq(None),
            )
            .returning(|_, _, _| Ok(Some(create_test_entity())));

        // Create service with proper mocks
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
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
        let mut class_repo = MockClassDefinitionRepo::new();

        let entity = create_test_entity();

        // Setup mock class definition repository to return a published class definition
        class_repo
            .expect_get_by_entity_type()
            .with(predicate::eq("test_entity"))
            .returning(|_| Ok(Some(create_test_class_definition())));

        // Setup mock repository response
        repo.expect_create()
            .with(predicate::function(|e: &DynamicEntity| {
                e.entity_type == "test_entity"
            }))
            .returning(|_| Ok(()));

        // Create service with proper mocks
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
        let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

        let result = service.create_entity(&entity).await;

        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_entity_missing_required_field() -> Result<()> {
        let mut repo = MockDynamicEntityRepo::new();
        let mut class_repo = MockClassDefinitionRepo::new();

        // Setup mock class definition repository
        class_repo
            .expect_get_by_entity_type()
            .with(predicate::eq("test_entity"))
            .returning(move |_| Ok(Some(create_test_class_definition())));

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
            definition: Arc::new(create_test_class_definition()),
        };

        // Create service with proper mocks
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
        let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

        // Try to create the entity, should fail because of missing required field
        let result = service.create_entity(&entity).await;

        // Check that we got a validation error
        assert!(result.is_err());
        match result {
            Err(Error::Validation(msg)) => {
                assert!(msg.contains("Required field 'name' is missing"));
            }
            _ => panic!("Expected validation error, got: {:?}", result),
        }

        Ok(())
    }

    #[test]
    fn test_get_entity_by_uuid_sync() {
        let mut repo = MockDynamicEntityRepo::new();
        let mut class_repo = MockClassDefinitionRepo::new();
        let entity_type = "test_entity";

        // Create a UUID that lives for the entire test function
        let uuid = Uuid::nil();

        repo.expect_get_by_type()
            .with(eq(entity_type), eq(uuid.clone()), eq(None))
            .returning(|_, _, _| Ok(Some(create_test_entity())));

        class_repo
            .expect_get_by_entity_type()
            .with(eq(entity_type))
            .returning(|_| Ok(Some(create_test_class_definition())));

        // Create service with proper mocks
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
        let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(service.get_entity_by_uuid(entity_type, &uuid, None));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_entity_by_type() -> Result<()> {
        let mut repo = MockDynamicEntityRepo::new();
        let mut class_repo = MockClassDefinitionRepo::new();

        let entity_type = "test_entity";

        // Setup mock class definition repository
        class_repo
            .expect_get_by_entity_type()
            .with(predicate::eq(entity_type))
            .returning(|_| Ok(Some(create_test_class_definition())));

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
        let class_service = ClassDefinitionService::new(Arc::new(class_repo));
        let service = DynamicEntityService::new(Arc::new(repo), Arc::new(class_service));

        let entities = service.list_entities(entity_type, 10, 0, None).await?;

        assert!(!entities.is_empty());
        assert_eq!(entities[0].entity_type, entity_type);

        Ok(())
    }
}
