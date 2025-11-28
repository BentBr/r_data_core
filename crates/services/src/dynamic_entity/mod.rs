#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod crud;
mod filtering;
mod validation;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::entity_definition::EntityDefinitionService;
use r_data_core_persistence::DynamicEntityRepositoryTrait;

/// Service for managing dynamic entities with validation based on entity definitions
#[derive(Clone)]
pub struct DynamicEntityService {
    repository: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
    entity_definition_service: Arc<EntityDefinitionService>,
}

impl DynamicEntityService {
    /// Create a new `DynamicEntityService` with the provided repository and entity definition service
    ///
    /// # Arguments
    /// * `repository` - Repository for dynamic entities
    /// * `entity_definition_service` - Service for entity definitions
    #[must_use]
    pub fn new(
        repository: Arc<dyn DynamicEntityRepositoryTrait + Send + Sync>,
        entity_definition_service: Arc<EntityDefinitionService>,
    ) -> Self {
        Self {
            repository,
            entity_definition_service,
        }
    }

    /// Get the underlying repository - helper for debugging
    #[must_use]
    pub fn get_repository(&self) -> &Arc<dyn DynamicEntityRepositoryTrait + Send + Sync> {
        &self.repository
    }

    /// Expose the entity definition service (needed by worker-to-entity integration)
    #[must_use]
    pub fn entity_definition_service(&self) -> Arc<EntityDefinitionService> {
        self.entity_definition_service.clone()
    }
}
