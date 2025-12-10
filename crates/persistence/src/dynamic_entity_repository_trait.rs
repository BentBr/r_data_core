use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

/// Parameters for filtering entities
#[derive(Debug, Clone)]
pub struct FilterEntitiesParams {
    /// Maximum number of entities to return
    pub limit: i64,
    /// Number of entities to skip
    pub offset: i64,
    /// Field filters as key-value pairs
    pub filters: Option<HashMap<String, JsonValue>>,
    /// Filter operators: field name -> operator (e.g., "=", ">", "<", "<=", ">=", "IN", "NOT IN")
    /// If not provided, defaults to "=" for all filters
    pub filter_operators: Option<HashMap<String, String>>,
    /// Search parameters: (`search_term`, `fields_to_search`)
    pub search: Option<(String, Vec<String>)>,
    /// Sort parameters: (field, direction)
    pub sort: Option<(String, String)>,
    /// Fields to include in the result
    pub fields: Option<Vec<String>>,
}

impl FilterEntitiesParams {
    /// Create a new `FilterEntitiesParams` with default pagination
    #[must_use]
    pub const fn new(limit: i64, offset: i64) -> Self {
        Self {
            limit,
            offset,
            filters: None,
            filter_operators: None,
            search: None,
            sort: None,
            fields: None,
        }
    }

    /// Set filters
    #[must_use]
    pub fn with_filters(mut self, filters: Option<HashMap<String, JsonValue>>) -> Self {
        self.filters = filters;
        self
    }

    /// Set filter operators
    #[must_use]
    pub fn with_filter_operators(mut self, operators: Option<HashMap<String, String>>) -> Self {
        self.filter_operators = operators;
        self
    }

    /// Set search parameters
    #[must_use]
    pub fn with_search(mut self, search: Option<(String, Vec<String>)>) -> Self {
        self.search = search;
        self
    }

    /// Set sort parameters
    #[must_use]
    pub fn with_sort(mut self, sort: Option<(String, String)>) -> Self {
        self.sort = sort;
        self
    }

    /// Set fields to include
    #[must_use]
    pub fn with_fields(mut self, fields: Option<Vec<String>>) -> Self {
        self.fields = fields;
        self
    }
}

/// Trait defining the contract for dynamic entity repositories
#[async_trait::async_trait]
pub trait DynamicEntityRepositoryTrait: Send + Sync {
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
        params: &FilterEntitiesParams,
    ) -> Result<Vec<DynamicEntity>>;

    /// Count entities of a specific type
    async fn count_entities(&self, entity_type: &str) -> Result<i64>;
}
