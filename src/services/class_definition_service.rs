use crate::entity::class::definition::ClassDefinition;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Service for managing class definitions
pub struct ClassDefinitionService {
    repository: Arc<dyn ClassDefinitionRepositoryTrait>,
}

impl ClassDefinitionService {
    /// Create a new class definition service
    pub fn new(repository: Arc<dyn ClassDefinitionRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// List class definitions with pagination
    pub async fn list_class_definitions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClassDefinition>> {
        self.repository.list(limit, offset).await
    }

    /// Get a class definition by UUID
    pub async fn get_class_definition(&self, uuid: &Uuid) -> Result<ClassDefinition> {
        let definition = self.repository.get_by_uuid(uuid).await?;
        definition.ok_or_else(|| {
            Error::NotFound(format!("Class definition with UUID {} not found", uuid))
        })
    }

    /// Get a class definition by entity type
    pub async fn get_class_definition_by_entity_type(
        &self,
        entity_type: &str,
    ) -> Result<ClassDefinition> {
        let definition = self.repository.get_by_entity_type(entity_type).await?;
        definition.ok_or_else(|| {
            Error::NotFound(format!(
                "Class definition with entity type '{}' not found",
                entity_type
            ))
        })
    }

    /// Create a new class definition
    pub async fn create_class_definition(&self, definition: &ClassDefinition) -> Result<Uuid> {
        // Validate that entity type follows naming conventions
        self.validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        self.validate_fields(&definition)?;

        // Create the class definition
        let uuid = self.repository.create(definition).await?;

        // Create or update the database schema for this entity type
        self.repository
            .update_entity_view_for_class_definition(definition)
            .await?;

        Ok(uuid)
    }

    /// Update an existing class definition
    pub async fn update_class_definition(
        &self,
        uuid: &Uuid,
        definition: &ClassDefinition,
    ) -> Result<()> {
        // Check that the class definition exists
        let existing = self.repository.get_by_uuid(uuid).await?;
        if existing.is_none() {
            return Err(Error::NotFound(format!(
                "Class definition with UUID {} not found",
                uuid
            )));
        }

        // Validate that entity type follows naming conventions
        self.validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        self.validate_fields(&definition)?;

        // Update the class definition
        self.repository.update(uuid, definition).await?;

        // Update the database schema for this entity type
        self.repository
            .update_entity_view_for_class_definition(definition)
            .await?;

        Ok(())
    }

    /// Delete a class definition
    pub async fn delete_class_definition(&self, uuid: &Uuid) -> Result<()> {
        // Check number of records in the entity table
        let definition = self.repository.get_by_uuid(uuid).await?;

        if let Some(def) = definition {
            let table_name = def.get_table_name();
            let table_exists = self.repository.check_view_exists(&table_name).await?;

            if table_exists {
                let record_count = self.repository.count_view_records(&table_name).await?;

                if record_count > 0 {
                    return Err(Error::Validation(format!(
                        "Cannot delete class definition that has {} entities. Delete all entities first.",
                        record_count
                    )));
                }
            }

            // Delete the class definition and associated tables
            self.repository.delete(uuid).await?;

            Ok(())
        } else {
            Err(Error::NotFound(format!(
                "Class definition with UUID {} not found",
                uuid
            )))
        }
    }

    /// Validate entity type name
    fn validate_entity_type(&self, entity_type: &str) -> Result<()> {
        // Entity type must be alphanumeric with underscores, starting with a letter
        let valid_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

        if !valid_pattern.is_match(entity_type) {
            return Err(Error::Validation(format!(
                "Entity type '{}' must start with a letter and contain only letters, numbers, and underscores",
                entity_type
            )));
        }

        // Check reserved words
        let reserved_words = [
            "user",
            "group",
            "role",
            "permission",
            "session",
            "token",
            "class",
            "entity",
            "public",
            "private",
            "protected",
            "table",
            "column",
            "row",
            "index",
            "view",
            "schema",
        ];

        if reserved_words.contains(&entity_type.to_lowercase().as_str()) {
            return Err(Error::Validation(format!(
                "Entity type '{}' is a reserved word",
                entity_type
            )));
        }

        Ok(())
    }

    /// Validate field definitions
    fn validate_fields(&self, definition: &ClassDefinition) -> Result<()> {
        // Check for duplicate field names
        let mut field_names = HashMap::new();

        for field in &definition.fields {
            if let Some(existing) = field_names.get(&field.name.to_lowercase()) {
                return Err(Error::Validation(format!(
                    "Duplicate field name '{}' (previously defined at position {})",
                    field.name, existing
                )));
            }

            field_names.insert(field.name.to_lowercase(), field_names.len() + 1);
        }

        // Validate each field
        for field in &definition.fields {
            // Field name must be alphanumeric with underscores, starting with a letter
            let valid_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

            if !valid_pattern.is_match(&field.name) {
                return Err(Error::Validation(format!(
                    "Field name '{}' must start with a letter and contain only letters, numbers, and underscores",
                    field.name
                )));
            }

            // Additional field-specific validations can be added here
        }

        Ok(())
    }

    /// Cleanup unused entity tables
    pub async fn cleanup_unused_entity_tables(&self) -> Result<()> {
        self.repository.cleanup_unused_entity_view().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::field::types::FieldType;
    use crate::entity::field::FieldDefinition;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::{self, eq};
    use std::collections::HashMap;

    // Create a mock for the repository trait
    mock! {
        pub ClassDefinitionRepo {}

        #[async_trait]
        impl ClassDefinitionRepositoryTrait for ClassDefinitionRepo {
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
        let creator_id = Uuid::now_v7();

        let mut definition = ClassDefinition::new(
            "TestEntity".to_string(),
            "Test Entity".to_string(),
            Some("A test entity type".to_string()),
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
                    ui_settings: Default::default(),
                    constraints: Default::default(),
                    validation: Default::default(),
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
                    ui_settings: Default::default(),
                    constraints: Default::default(),
                    validation: Default::default(),
                },
            ],
            creator_id,
        );

        definition.uuid = Uuid::now_v7();
        definition
    }

    #[tokio::test]
    async fn test_get_class_definition_by_uuid_found() -> Result<()> {
        let mut mock_repo = MockClassDefinitionRepo::new();
        let uuid = Uuid::now_v7();
        let expected_definition = create_test_class_definition();

        mock_repo
            .expect_get_by_uuid()
            .withf(move |id| id == &uuid)
            .returning(move |_| Ok(Some(expected_definition.clone())));

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.get_class_definition(&uuid).await?;

        assert_eq!(result.entity_type, "TestEntity");
        assert_eq!(result.display_name, "Test Entity");
        assert_eq!(result.fields.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_class_definition_by_uuid_not_found() -> Result<()> {
        let mut mock_repo = MockClassDefinitionRepo::new();
        let uuid = Uuid::now_v7();

        mock_repo
            .expect_get_by_uuid()
            .withf(move |id| id == &uuid)
            .returning(|_| Ok(None));

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.get_class_definition(&uuid).await;

        assert!(result.is_err());
        if let Err(Error::NotFound(msg)) = result {
            assert!(msg.contains(&uuid.to_string()));
        } else {
            panic!("Expected NotFound error");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_class_definition_success() -> Result<()> {
        let mut mock_repo = MockClassDefinitionRepo::new();
        let definition = create_test_class_definition();
        let expected_uuid = definition.uuid;

        mock_repo
            .expect_create()
            .with(predicate::always())
            .returning(move |_| Ok(expected_uuid));

        mock_repo
            .expect_update_entity_view_for_class_definition()
            .with(predicate::always())
            .returning(|_| Ok(()));

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_class_definition(&definition).await?;

        assert_eq!(result, expected_uuid);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_class_definition_invalid_entity_type() -> Result<()> {
        let mock_repo = MockClassDefinitionRepo::new();
        let mut definition = create_test_class_definition();
        definition.entity_type = "123InvalidStart".to_string();

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_class_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("must start with a letter"));
        } else {
            panic!("Expected Validation error");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_class_definition_reserved_word() -> Result<()> {
        let mock_repo = MockClassDefinitionRepo::new();
        let mut definition = create_test_class_definition();
        definition.entity_type = "table".to_string();

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_class_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("reserved word"));
        } else {
            panic!("Expected Validation error");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_class_definition_duplicate_field_names() -> Result<()> {
        let mock_repo = MockClassDefinitionRepo::new();
        let mut definition = create_test_class_definition();

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
            ui_settings: Default::default(),
            constraints: Default::default(),
            validation: Default::default(),
        });

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_class_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("Duplicate field name"));
        } else {
            panic!("Expected Validation error");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_class_definition_with_records() -> Result<()> {
        let mut mock_repo = MockClassDefinitionRepo::new();
        let uuid = Uuid::now_v7();
        let definition = create_test_class_definition();

        mock_repo
            .expect_get_by_uuid()
            .withf(move |id| id == &uuid)
            .returning(move |_| Ok(Some(definition.clone())));

        mock_repo.expect_check_view_exists().returning(|_| Ok(true));

        mock_repo.expect_count_view_records().returning(|_| Ok(10)); // 10 records exist

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.delete_class_definition(&uuid).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("Cannot delete class definition that has 10 entities"));
        } else {
            panic!("Expected Validation error");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_class_definition_success() -> Result<()> {
        let mut mock_repo = MockClassDefinitionRepo::new();
        let uuid = Uuid::now_v7();
        let definition = create_test_class_definition();

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

        let service = ClassDefinitionService::new(Arc::new(mock_repo));
        let result = service.delete_class_definition(&uuid).await;

        assert!(result.is_ok());

        Ok(())
    }
}
