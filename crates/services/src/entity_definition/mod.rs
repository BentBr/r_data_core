#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod cache;
mod crud;
mod fields;
mod schema;
mod validation;

#[cfg(test)]
mod tests;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use std::sync::Arc;

/// Service for managing entity definitions
#[derive(Clone)]
pub struct EntityDefinitionService {
    repository: Arc<dyn EntityDefinitionRepositoryTrait>,
    cache_manager: Arc<CacheManager>,
}

/// Helper structure describing an entity field (including system fields)
#[derive(Debug, Clone)]
pub struct ServiceEntityFieldInfo {
    /// Field name
    pub name: String,
    /// Field type as string
    pub field_type: String,
    /// Whether the field is required
    pub required: bool,
    /// Whether the field is a system field
    pub system: bool,
}

impl EntityDefinitionService {
    /// Create a new entity definition service
    ///
    /// # Arguments
    /// * `repository` - Repository for entity definitions
    /// * `cache_manager` - Cache manager for caching definitions
    #[must_use]
    pub fn new(
        repository: Arc<dyn EntityDefinitionRepositoryTrait>,
        cache_manager: Arc<CacheManager>,
    ) -> Self {
        Self {
            repository,
            cache_manager,
        }
    }

    /// Create a new entity definition service with disabled cache (for testing)
    ///
    /// # Arguments
    /// * `repository` - Repository for entity definitions
    #[must_use]
    pub fn new_without_cache(repository: Arc<dyn EntityDefinitionRepositoryTrait>) -> Self {
        use r_data_core_core::config::CacheConfig;
        let config = CacheConfig {
            enabled: false,
            ttl: 3600,
            max_size: 10000,
            entity_definition_ttl: 0,
            api_key_ttl: 600,
        };
        Self {
            repository,
            cache_manager: Arc::new(CacheManager::new(config)),
        }
    }
}
