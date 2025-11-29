#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;

use r_data_core_core::error::Result;
use r_data_core_core::public_api::AdvancedEntityQuery;
use r_data_core_core::DynamicEntity;

/// Trait for dynamic entity query repository operations
#[async_trait]
pub trait DynamicEntityQueryRepositoryTrait: Send + Sync {
    /// Query dynamic entity instances with advanced filtering
    ///
    /// # Arguments
    /// * `entity_type` - Type of entity to query
    /// * `query` - Advanced query parameters
    ///
    /// # Errors
    /// Returns an error if the query cannot be executed
    async fn query_entities(
        &self,
        entity_type: &str,
        query: &AdvancedEntityQuery,
    ) -> Result<Vec<DynamicEntity>>;
}
