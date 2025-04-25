use crate::entity::class::definition::ClassDefinition;
use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

/// Repository trait for class definition operations
#[async_trait]
pub trait ClassDefinitionRepositoryTrait: Send + Sync {
    /// List all class definitions with pagination
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>>;

    /// Get a class definition by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<ClassDefinition>>;

    /// Get a class definition by entity type
    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>>;

    /// Create a new class definition
    async fn create(&self, definition: &ClassDefinition) -> Result<Uuid>;

    /// Update an existing class definition
    async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()>;

    /// Delete a class definition
    async fn delete(&self, uuid: &Uuid) -> Result<()>;

    /// Apply schema SQL to database
    async fn apply_schema(&self, schema_sql: &str) -> Result<()>;

    /// Update entity view for class definition
    async fn update_entity_view_for_class_definition(
        &self,
        class_definition: &ClassDefinition,
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
