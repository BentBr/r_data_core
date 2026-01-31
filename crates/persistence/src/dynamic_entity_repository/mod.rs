use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::dynamic_entity_repository_trait::{DynamicEntityRepositoryTrait, FilterEntitiesParams};
use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

mod create;
mod filter;
mod query;
mod update;

use create::create_entity;
use filter::filter_entities_impl;
use query::{
    count_children_impl, count_entities_impl, delete_by_type_impl, find_one_by_filters_impl,
    get_all_by_type_impl, get_by_type_impl, has_children_impl, query_by_parent_impl,
    query_by_path_impl,
};
use update::update_entity;

/// Repository for managing dynamic entities
pub struct DynamicEntityRepository {
    /// Database connection pool
    pub pool: PgPool,
    /// Cache manager for entity definitions
    pub cache_manager: Option<Arc<CacheManager>>,
}

impl DynamicEntityRepository {
    /// Create a new repository instance
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache_manager: None,
        }
    }

    /// Create a new repository instance with cache manager
    #[must_use]
    pub const fn with_cache(pool: PgPool, cache_manager: Arc<CacheManager>) -> Self {
        Self {
            pool,
            cache_manager: Some(cache_manager),
        }
    }

    /// Create a new dynamic entity
    ///
    /// # Errors
    /// Returns an error if the database operation fails or validation fails
    /// Returns the UUID
    pub async fn create(&self, entity: &DynamicEntity) -> Result<Uuid> {
        create_entity(self, entity).await
    }

    /// Update an existing dynamic entity
    ///
    /// # Errors
    /// Returns an error if the database operation fails or validation fails
    pub async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        update_entity(self, entity).await
    }

    /// Count entities of a specific type
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        count_entities_impl(self, entity_type).await
    }

    /// Query entities by `parent_uuid`
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn query_by_parent(
        &self,
        entity_type: &str,
        parent_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>> {
        query_by_parent_impl(self, entity_type, parent_uuid, limit, offset).await
    }

    /// Query entities by exact `path`
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn query_by_path(
        &self,
        entity_type: &str,
        path: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>> {
        query_by_path_impl(self, entity_type, path, limit, offset).await
    }

    /// Check if an entity has children
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn has_children(&self, parent_uuid: &Uuid) -> Result<bool> {
        has_children_impl(self, parent_uuid).await
    }

    /// Count children for an entity
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_children(&self, parent_uuid: &Uuid) -> Result<i64> {
        count_children_impl(self, parent_uuid).await
    }

    /// Find a single entity by filters
    ///
    /// # Arguments
    /// * `entity_type` - Type of entity to find
    /// * `filters` - Map of field names to values for filtering
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn find_one_by_filters(
        &self,
        entity_type: &str,
        filters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<Option<DynamicEntity>> {
        find_one_by_filters_impl(self, entity_type, filters).await
    }
}

#[async_trait::async_trait]
impl DynamicEntityRepositoryTrait for DynamicEntityRepository {
    async fn create(&self, entity: &DynamicEntity) -> Result<Uuid> {
        self.create(entity).await
    }

    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        self.update(entity).await
    }

    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        get_by_type_impl(self, entity_type, uuid, exclusive_fields).await
    }

    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        get_all_by_type_impl(self, entity_type, limit, offset, exclusive_fields).await
    }

    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        delete_by_type_impl(self, entity_type, uuid).await
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        params: &FilterEntitiesParams,
    ) -> Result<Vec<DynamicEntity>> {
        filter_entities_impl(self, entity_type, params).await
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        self.count_entities(entity_type).await
    }

    async fn count_children(&self, parent_uuid: &Uuid) -> Result<i64> {
        self.count_children(parent_uuid).await
    }
}
