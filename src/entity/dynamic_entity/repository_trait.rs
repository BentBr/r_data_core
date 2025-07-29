use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::error::Result;

/// Trait defining the contract for dynamic entity repositories
#[async_trait::async_trait]
pub trait DynamicEntityRepositoryTrait {
    /// Get all entities of a specific type with pagination
    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>>;

    /// Get a specific entity by type and UUID
    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>>;

    /// Create a new dynamic entity
    async fn create(&self, entity: &DynamicEntity) -> Result<()>;

    /// Update an existing dynamic entity
    async fn update(&self, entity: &DynamicEntity) -> Result<()>;

    /// Delete a dynamic entity by type and UUID
    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()>;

    /// Filter entities by field values with advanced options
    async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>>;

    /// Count entities of a specific type
    async fn count_entities(&self, entity_type: &str) -> Result<i64>;
}
