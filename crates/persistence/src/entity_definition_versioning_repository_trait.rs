#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity_definition_versioning_repository::{
    EntityDefinitionVersionMeta, EntityDefinitionVersionPayload,
};
use r_data_core_core::error::Result;

/// Trait for entity definition versioning repository operations
#[async_trait]
pub trait EntityDefinitionVersioningRepositoryTrait: Send + Sync {
    /// Create a pre-update snapshot for an entity definition
    /// The snapshot's created_by is extracted from the JSON data (updated_by or created_by).
    ///
    /// # Arguments
    /// * `definition_uuid` - UUID of the entity definition
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn snapshot_pre_update(&self, definition_uuid: Uuid) -> Result<()>;

    /// List all versions for an entity definition
    ///
    /// # Arguments
    /// * `definition_uuid` - UUID of the entity definition
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_definition_versions(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Vec<EntityDefinitionVersionMeta>>;

    /// Get a specific version of an entity definition
    ///
    /// # Arguments
    /// * `definition_uuid` - UUID of the entity definition
    /// * `version_number` - Version number to retrieve
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_definition_version(
        &self,
        definition_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityDefinitionVersionPayload>>;

    /// Get current entity definition metadata
    ///
    /// # Arguments
    /// * `definition_uuid` - UUID of the entity definition
    ///
    /// # Returns
    /// Tuple of (`version`, `updated_at`, `updated_by`, `updated_by_name`)
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_current_definition_metadata(
        &self,
        definition_uuid: Uuid,
    ) -> Result<Option<(i32, OffsetDateTime, Option<Uuid>, Option<String>)>>;

    /// Prune entity definition versions older than the specified number of days
    ///
    /// # Arguments
    /// * `days` - Number of days to keep
    ///
    /// # Returns
    /// Number of versions deleted
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn prune_older_than_days(&self, days: i32) -> Result<u64>;

    /// Prune entity definition versions, keeping only the latest N versions per definition
    ///
    /// # Arguments
    /// * `keep` - Number of latest versions to keep per definition
    ///
    /// # Returns
    /// Number of versions deleted
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn prune_keep_latest_per_definition(&self, keep: i32) -> Result<u64>;
}

