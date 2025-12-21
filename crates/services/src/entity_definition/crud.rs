#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use uuid::Uuid;

use super::EntityDefinitionService;

impl EntityDefinitionService {
    /// List entity definitions with pagination
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_entity_definitions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EntityDefinition>> {
        self.repository.list(limit, offset).await
    }

    /// Count entity definitions
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count_entity_definitions(&self) -> Result<i64> {
        self.repository.count().await
    }

    /// Get an entity definition by UUID
    ///
    /// # Errors
    /// Returns an error if the entity definition is not found or database query fails
    pub async fn get_entity_definition(&self, uuid: &Uuid) -> Result<EntityDefinition> {
        // Check cache first
        let cache_key = Self::cache_key_by_uuid(uuid);
        if let Ok(Some(cached)) = self.cache_manager.get::<EntityDefinition>(&cache_key).await {
            return Ok(cached);
        }

        // Cache miss - query repository
        let Some(definition) = self.repository.get_by_uuid(uuid).await? else {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with UUID {uuid} not found"
            )));
        };

        // Cache the result (no TTL - cache until explicitly invalidated)
        if let Err(e) = self.cache_manager.set(&cache_key, &definition, None).await {
            log::warn!("Failed to cache entity definition: {e}");
        }

        Ok(definition)
    }

    /// Get an entity definition by entity type
    ///
    /// # Errors
    /// Returns an error if the entity definition is not found or database query fails
    pub async fn get_entity_definition_by_entity_type(
        &self,
        entity_type: &str,
    ) -> Result<EntityDefinition> {
        // Check cache first
        let cache_key = Self::cache_key_by_entity_type(entity_type);
        if let Ok(Some(cached)) = self.cache_manager.get::<EntityDefinition>(&cache_key).await {
            return Ok(cached);
        }

        // Cache miss - query repository
        let Some(definition) = self.repository.get_by_entity_type(entity_type).await? else {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with entity type '{entity_type}' not found"
            )));
        };

        // Cache the result with both keys (no TTL - cache until explicitly invalidated)
        let type_key = Self::cache_key_by_entity_type(entity_type);
        let uuid_key = Self::cache_key_by_uuid(&definition.uuid);

        if let Err(e) = self.cache_manager.set(&type_key, &definition, None).await {
            log::warn!("Failed to cache entity definition by type: {e}");
        }
        if let Err(e) = self.cache_manager.set(&uuid_key, &definition, None).await {
            log::warn!("Failed to cache entity definition by UUID: {e}");
        }

        Ok(definition)
    }

    /// Create a new entity definition
    ///
    /// # Errors
    /// Returns an error if validation fails, entity type already exists, or creation fails
    pub async fn create_entity_definition(&self, definition: &EntityDefinition) -> Result<Uuid> {
        // Validate that entity type follows naming conventions
        Self::validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        Self::validate_fields(definition)?;

        // Check for duplicate entity type
        let existing = self
            .repository
            .get_by_entity_type(&definition.entity_type)
            .await?;
        if existing.is_some() {
            return Err(r_data_core_core::error::Error::ClassAlreadyExists(format!(
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

        // Fetch the created definition from the database to get the correct UUID and all fields
        let created_definition = self.repository.get_by_uuid(&uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with UUID {uuid} not found after creation"
            ))
        })?;

        // Cache the new definition with both keys (using the fetched definition with correct UUID)
        let type_key = Self::cache_key_by_entity_type(&created_definition.entity_type);
        let uuid_key = Self::cache_key_by_uuid(&uuid);

        if let Err(e) = self
            .cache_manager
            .set(&type_key, &created_definition, None)
            .await
        {
            log::warn!("Failed to cache new entity definition by type: {e}");
        }
        if let Err(e) = self
            .cache_manager
            .set(&uuid_key, &created_definition, None)
            .await
        {
            log::warn!("Failed to cache new entity definition by UUID: {e}");
        }

        Ok(uuid)
    }

    /// Update an existing entity definition
    ///
    /// # Errors
    /// Returns an error if the entity definition is not found, validation fails, or update fails
    pub async fn update_entity_definition(
        &self,
        uuid: &Uuid,
        definition: &EntityDefinition,
    ) -> Result<()> {
        // Check that the entity definition exists and get old entity_type
        let Some(existing) = self.repository.get_by_uuid(uuid).await? else {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with UUID {uuid} not found"
            )));
        };
        let old_entity_type = existing.entity_type.clone();

        // Validate that entity type follows naming conventions
        Self::validate_entity_type(&definition.entity_type)?;

        // Validate field names and configurations
        Self::validate_fields(definition)?;

        // Invalidate old cache entries before update
        self.invalidate_entity_definition_cache(&old_entity_type, uuid)
            .await?;

        // Update the entity definition
        self.repository.update(uuid, definition).await?;

        // Update the database schema for this entity type
        self.repository
            .update_entity_view_for_entity_definition(definition)
            .await?;

        // Fetch the updated definition from the database to get the correct state
        let updated_definition = self.repository.get_by_uuid(uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with UUID {uuid} not found after update"
            ))
        })?;

        // Cache the updated definition with both keys (using the fetched definition)
        let type_key = Self::cache_key_by_entity_type(&updated_definition.entity_type);
        let uuid_key = Self::cache_key_by_uuid(uuid);

        if let Err(e) = self
            .cache_manager
            .set(&type_key, &updated_definition, None)
            .await
        {
            log::warn!("Failed to cache updated entity definition by type: {e}");
        }
        if let Err(e) = self
            .cache_manager
            .set(&uuid_key, &updated_definition, None)
            .await
        {
            log::warn!("Failed to cache updated entity definition by UUID: {e}");
        }

        // If entity_type changed, also invalidate the old entity_type cache key
        if old_entity_type != definition.entity_type {
            let old_type_key = Self::cache_key_by_entity_type(&old_entity_type);
            if let Err(e) = self.cache_manager.delete(&old_type_key).await {
                log::warn!("Failed to invalidate old entity type cache key: {e}");
            }
        }

        Ok(())
    }

    /// Delete an entity definition
    ///
    /// # Errors
    /// Returns an error if the entity definition is not found, has existing entities, or deletion fails
    pub async fn delete_entity_definition(&self, uuid: &Uuid) -> Result<()> {
        // Check number of records in the entity table
        let Some(def) = self.repository.get_by_uuid(uuid).await? else {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "Entity definition with UUID {uuid} not found"
            )));
        };

        let entity_type = def.entity_type.clone();
        let table_name = def.get_table_name();
        let table_exists = self.repository.check_view_exists(&table_name).await?;

        if table_exists {
            let record_count = self.repository.count_view_records(&table_name).await?;

            if record_count > 0 {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Cannot delete entity definition that has {record_count} entities. Delete all entities first."
                )));
            }
        }

        // Delete the entity definition and associated tables
        self.repository.delete(uuid).await?;

        // Invalidate cache entries after successful deletion
        self.invalidate_entity_definition_cache(&entity_type, uuid)
            .await?;

        Ok(())
    }
}
