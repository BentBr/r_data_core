use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::error::Result;

/// Trait defining the interface for DynamicEntityRepository implementations
#[async_trait::async_trait]
pub trait DynamicEntityRepositoryTrait: Send + Sync {
    /// Create a new dynamic entity
    async fn create(&self, entity: &DynamicEntity) -> Result<()>;

    /// Update an existing dynamic entity
    async fn update(&self, entity: &DynamicEntity) -> Result<()>;

    /// Get a dynamic entity by type
    async fn get_by_type(&self, entity_type: &str) -> Result<Option<DynamicEntity>>;

    /// Get all entities of a specific type
    async fn get_all_by_type(&self, entity_type: &str) -> Result<Vec<DynamicEntity>>;

    /// Delete an entity by type
    async fn delete_by_type(&self, entity_type: &str) -> Result<()>;

    /// Filter entities by field values
    async fn filter_entities(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>>;

    /// Count entities of a specific type
    async fn count_entities(&self, entity_type: &str) -> Result<i64>;
}
