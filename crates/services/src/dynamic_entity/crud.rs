#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use uuid::Uuid;

use super::DynamicEntityService;

impl DynamicEntityService {
    /// Check if the entity type exists and is published - a common check for all operations
    ///
    /// # Arguments
    /// * `entity_type` - Entity type string
    ///
    /// # Errors
    /// Returns an error if the entity type is not found or not published
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
    /// Returns an error if the entity type is not found, not published, or the database query fails
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
    /// Returns an error if the entity type is not found, not published, or the database query fails
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        self.repository.count_entities(entity_type).await
    }

    /// Get an entity by UUID
    ///
    /// # Errors
    /// Returns an error if the entity type is not found, not published, or the database query fails
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

    /// Get an entity by UUID with optional children count
    ///
    /// # Errors
    /// Returns an error if the entity type is not found, not published, or the database query fails
    pub async fn get_entity_by_uuid_with_children_count(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
        include_children_count: bool,
    ) -> Result<(Option<DynamicEntity>, Option<i64>)> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        let entity = self
            .repository
            .get_by_type(entity_type, uuid, exclusive_fields)
            .await?;

        let children_count = if include_children_count && entity.is_some() {
            Some(self.repository.count_children(uuid).await?)
        } else {
            None
        };

        Ok((entity, children_count))
    }

    /// Get an entity by UUID without knowing the entity type
    ///
    /// This searches across all entity types. Useful when you have a UUID
    /// but don't know which entity type it belongs to (e.g., `parent_uuid` lookup).
    ///
    /// # Errors
    /// Returns an error if the entity is not found or the database query fails
    pub async fn get_entity_by_uuid_any_type(&self, uuid: Uuid) -> Result<DynamicEntity> {
        self.repository
            .get_by_uuid_any_type(&uuid)
            .await?
            .ok_or_else(|| {
                r_data_core_core::error::Error::NotFound(format!(
                    "Entity with UUID {uuid} not found"
                ))
            })
    }

    /// Create a new entity with validation
    ///
    /// # Errors
    /// Returns an error if the validation fails, the entity type is not found/not published, or creation fails
    /// Returns the UUID
    pub async fn create_entity(&self, entity: &DynamicEntity) -> Result<Uuid> {
        // Check if the entity type is published
        self.check_entity_type_exists_and_published(&entity.entity_type)
            .await?;

        // Validate entity against entity definition
        Self::validate_entity(entity)?;

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
        Self::validate_entity(entity)?;

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
        Self::validate_entity(entity)?;

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

    /// Find a single entity by field filters
    ///
    /// # Errors
    /// Returns an error if the query fails
    pub async fn find_one_by_filters(
        &self,
        entity_type: &str,
        filters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<Option<DynamicEntity>> {
        self.repository
            .find_one_by_filters(entity_type, filters)
            .await
    }

    /// Read a single raw field value, bypassing mapper redaction.
    /// Used for password verification.
    ///
    /// # Errors
    /// Returns an error if the query fails
    pub async fn get_raw_field_value(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        field_name: &str,
    ) -> Result<Option<String>> {
        self.repository
            .get_raw_field_value(entity_type, uuid, field_name)
            .await
    }
}
