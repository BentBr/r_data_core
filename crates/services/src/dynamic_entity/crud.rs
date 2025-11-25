#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::DynamicEntity;
use r_data_core_core::error::Result;
use uuid::Uuid;

use super::DynamicEntityService;

impl DynamicEntityService {
    /// Check if the entity type exists and is published - common check for all operations
    ///
    /// # Arguments
    /// * `entity_type` - Entity type string
    ///
    /// # Errors
    /// Returns an error if entity type is not found or not published
    pub(crate) async fn check_entity_type_exists_and_published(
        &self,
        entity_type: &str,
    ) -> Result<r_data_core_core::entity_definition::definition::EntityDefinition> {
        let entity_definition = self
            .entity_definition_service
            .get_entity_definition_by_entity_type(entity_type)
            .await?;

        if !entity_definition.published {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity type '{entity_type}' not found or not published"
            )));
        }

        Ok(entity_definition)
    }

    /// List entities with pagination
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
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
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository.count_entities(entity_type).await
    }

    /// Get an entity by UUID
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
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
    ///
    /// # Errors
    /// Returns an error if validation fails, entity type is not found/not published, or creation fails
    pub async fn create_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against entity definition
        self.validate_entity(entity)?;

        self.repository.create(entity).await
    }

    /// Update an existing entity with validation
    ///
    /// # Errors
    /// Returns an error if validation fails, entity type is not found/not published, or update fails
    pub async fn update_entity(&self, entity: &DynamicEntity) -> Result<()> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against entity definition
        self.validate_entity(entity)?;

        self.repository.update(entity).await
    }

    /// Update an existing entity with options (e.g., skip versioning snapshots)
    ///
    /// # Arguments
    /// * `entity` - Entity to update
    /// * `skip_versioning` - Whether to skip versioning snapshots
    ///
    /// # Errors
    /// Returns an error if validation fails, entity type is not found/not published, or update fails
    pub async fn update_entity_with_options(
        &self,
        entity: &DynamicEntity,
        skip_versioning: bool,
    ) -> Result<()> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against entity definition
        self.validate_entity(entity)?;

        if skip_versioning {
            // Temporary: inject internal flag until repository trait supports explicit param
            let mut cloned = entity.clone();
            cloned
                .field_data
                .insert("__skip_versioning".to_string(), serde_json::json!(true));
            self.repository.update(&cloned).await
        } else {
            self.repository.update(entity).await
        }
    }

    /// Delete an entity
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or deletion fails
    pub async fn delete_entity(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository.delete_by_type(entity_type, uuid).await
    }
}

