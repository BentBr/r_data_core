use crate::entity_definition::definition::EntityDefinition;
use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

/// Repository trait for entity definition operations
#[async_trait]
pub trait EntityDefinitionRepositoryTrait: Send + Sync {
    /// List all entity definitions with pagination
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>>;

    /// Count all entity definitions
    async fn count(&self) -> Result<i64>;

    /// Get an entity definition by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<EntityDefinition>>;

    /// Get an entity definition by entity type
    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>>;

    /// Create a new entity definition
    async fn create(&self, definition: &EntityDefinition) -> Result<Uuid>;

    /// Update an existing entity definition
    async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()>;

    /// Delete an entity definition
    async fn delete(&self, uuid: &Uuid) -> Result<()>;

    /// Apply schema SQL to database
    async fn apply_schema(&self, schema_sql: &str) -> Result<()>;

    /// Update entity view for entity definition
    async fn update_entity_view_for_entity_definition(
        &self,
        entity_definition: &EntityDefinition,
    ) -> Result<()>;

    /// Check if a view exists
    async fn check_view_exists(&self, view_name: &str) -> Result<bool>;

    /// Get view columns with their types
    async fn get_view_columns_with_types(&self, view_name: &str)
        -> Result<HashMap<String, String>>;

    /// Count records in a view
    async fn count_view_records(&self, view_name: &str) -> Result<i64>;

    /// Cleanup unused entity view
    async fn cleanup_unused_entity_view(&self) -> Result<()>;
}
