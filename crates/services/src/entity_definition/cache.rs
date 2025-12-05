#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use uuid::Uuid;

use super::EntityDefinitionService;

impl EntityDefinitionService {
    /// Generate cache key for entity definition by entity type
    #[must_use]
    pub(crate) fn cache_key_by_entity_type(entity_type: &str) -> String {
        format!("entity_def:by_type:{entity_type}")
    }

    /// Generate cache key for entity definition by UUID
    #[must_use]
    pub(crate) fn cache_key_by_uuid(uuid: &Uuid) -> String {
        format!("entity_def:by_uuid:{uuid}")
    }

    /// Invalidate cache entries for an entity definition
    ///
    /// # Arguments
    /// * `entity_type` - Entity type string
    /// * `uuid` - Entity definition UUID
    ///
    /// # Errors
    /// Returns an error if cache invalidation fails
    pub(crate) async fn invalidate_entity_definition_cache(
        &self,
        entity_type: &str,
        uuid: &Uuid,
    ) -> Result<()> {
        let type_key = Self::cache_key_by_entity_type(entity_type);
        let uuid_key = Self::cache_key_by_uuid(uuid);

        // Invalidate both cache keys
        if let Err(e) = self.cache_manager.delete(&type_key).await {
            log::warn!("Failed to invalidate entity type cache key: {e}");
        }
        if let Err(e) = self.cache_manager.delete(&uuid_key).await {
            log::warn!("Failed to invalidate UUID cache key: {e}");
        }

        Ok(())
    }
}
