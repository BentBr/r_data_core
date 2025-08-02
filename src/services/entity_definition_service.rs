use crate::entity::entity_definition::definition::EntityDefinition;
use crate::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use crate::entity::entity_definition::schema::Schema;
use crate::entity::field::types::FieldType;
use crate::entity::field::FieldDefinition;
use crate::error::{Error, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Service for managing entity definitions
#[derive(Clone)]
pub struct EntityDefinitionService {
    repository: Arc<dyn EntityDefinitionRepositoryTrait>,
}

impl EntityDefinitionService {
    /// Create a new entity definition service
    pub fn new(repository: Arc<dyn EntityDefinitionRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// List entity definitions with pagination
    pub async fn list_entity_definitions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EntityDefinition>> {
        self.repository.list(limit, offset).await
    }

    /// Get an entity definition by UUID
    pub async fn get_entity_definition(&self, uuid: &Uuid) -> Result<EntityDefinition> {
        let definition = self.repository.get_by_uuid(uuid).await?;
        definition.ok_or_else(|| {
            Error::NotFound(format!("Entity definition with UUID {} not found", uuid))
        })
    }

    /// Get an entity definition by entity type
    pub async fn get_entity_definition_by_entity_type(
        &self,
        entity_type: &str,
    ) -> Result<EntityDefinition> {
        let definition = self.repository.get_by_entity_type(entity_type).await?;
        definition.ok_or_else(|| {
            Error::NotFound(format!(
                "Entity definition with entity type '{}' not found",
                entity_type
            ))
        })
    }

    /// Create a new entity definition
    pub async fn create_entity_definition(&self, definition: &EntityDefinition) -> Result<Uuid> {
        // Validate that entity type follows naming conventions
        self.validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        self.validate_fields(&definition)?;

        // Check for duplicate entity type
        let existing = self
            .repository
            .get_by_entity_type(&definition.entity_type)
            .await?;
        if existing.is_some() {
            return Err(Error::ClassAlreadyExists(format!(
                "Entity type '{}' already exists",
                definition.entity_type
            )));
        }

        // Create the entity definition
        let uuid = self.repository.create(definition).await?;

        // Create or update the database schema for this entity type
        self.repository
            .update_entity_view_for_entity_definition(definition)
            .await?;

        Ok(uuid)
    }

    /// Update an existing entity definition
    pub async fn update_entity_definition(
        &self,
        uuid: &Uuid,
        definition: &EntityDefinition,
    ) -> Result<()> {
        // Check that the entity definition exists
        let existing = self.repository.get_by_uuid(uuid).await?;
        if existing.is_none() {
            return Err(Error::NotFound(format!(
                "Entity definition with UUID {} not found",
                uuid
            )));
        }

        // Validate that entity type follows naming conventions
        self.validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        self.validate_fields(&definition)?;

        // Update the entity definition
        self.repository.update(uuid, definition).await?;

        // Update the database schema for this entity type
        self.repository
            .update_entity_view_for_entity_definition(definition)
            .await?;

        Ok(())
    }

    /// Delete an entity definition
    pub async fn delete_entity_definition(&self, uuid: &Uuid) -> Result<()> {
        // Check number of records in the entity table
        let definition = self.repository.get_by_uuid(uuid).await?;

        if let Some(def) = definition {
            let table_name = def.get_table_name();
            let table_exists = self.repository.check_view_exists(&table_name).await?;

            if table_exists {
                let record_count = self.repository.count_view_records(&table_name).await?;

                if record_count > 0 {
                    return Err(Error::Validation(format!(
                        "Cannot delete entity definition that has {} entities. Delete all entities first.",
                        record_count
                    )));
                }
            }

            // Delete the entity definition and associated tables
            self.repository.delete(uuid).await?;

            Ok(())
        } else {
            Err(Error::NotFound(format!(
                "Entity definition with UUID {} not found",
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
            "class", "entity", "table", "column", "row", "index", "view", "schema",
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
    fn validate_fields(&self, definition: &EntityDefinition) -> Result<()> {
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

        // Field name must be alphanumeric with underscores, starting with a letter
        let valid_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

        // Validate each field
        for field in &definition.fields {
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

    /// Apply database schema for a specific entity definition or all if uuid is None
    pub async fn apply_schema(
        &self,
        uuid: Option<&Uuid>,
    ) -> Result<(i32, Vec<(String, Uuid, String)>)> {
        if let Some(id) = uuid {
            // Apply schema for a specific entity definition
            let definition = self.get_entity_definition(id).await?;

            match self
                .repository
                .update_entity_view_for_entity_definition(&definition)
                .await
            {
                Ok(_) => Ok((1, Vec::new())),
                Err(e) => {
                    let failed = vec![(
                        definition.entity_type.clone(),
                        definition.uuid,
                        e.to_string(),
                    )];
                    Ok((0, failed))
                }
            }
        } else {
            // Apply schema for all entity definitions
            let definitions = self.list_entity_definitions(1000, 0).await?;
            let mut success_count = 0;
            let mut failed = Vec::new();

            for definition in definitions {
                match self
                    .repository
                    .update_entity_view_for_entity_definition(&definition)
                    .await
                {
                    Ok(_) => {
                        success_count += 1;
                    }
                    Err(e) => {
                        failed.push((
                            definition.entity_type.clone(),
                            definition.uuid,
                            e.to_string(),
                        ));
                    }
                }
            }

            Ok((success_count, failed))
        }
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
    use time::OffsetDateTime;

    // Create a mock for the repository trait
    mock! {
        pub EntityDefinitionRepo {}

        #[async_trait]
        impl EntityDefinitionRepositoryTrait for EntityDefinitionRepo {
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>>;
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.get_entity_definition(&uuid).await;

        assert!(result.is_err());
        if let Err(Error::NotFound(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_entity_definition(&definition).await?;

        assert_eq!(result, expected_uuid);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_entity_definition_invalid_entity_type() -> Result<()> {
        let mock_repo = MockEntityDefinitionRepo::new();
        let mut definition = create_test_entity_definition();
        definition.entity_type = "123InvalidStart".to_string();

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_entity_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_entity_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
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
            ui_settings: Default::default(),
            constraints: Default::default(),
            validation: Default::default(),
        });

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.create_entity_definition(&definition).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.delete_entity_definition(&uuid).await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service
            .get_entity_definition_by_entity_type(entity_type)
            .await;

        assert!(result.is_err());
        if let Err(Error::NotFound(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.update_entity_definition(&uuid, &definition).await;

        assert!(result.is_err());
        if let Err(Error::NotFound(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service
            .update_entity_definition(&uuid, &invalid_definition)
            .await;

        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
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

        let service = EntityDefinitionService::new(Arc::new(mock_repo));
        let result = service.cleanup_unused_entity_tables().await;

        assert!(result.is_ok());

        Ok(())
    }
}
