#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Result;

// Re-export types for convenience
pub use crate::version_repository::{EntityVersionMeta, EntityVersionPayload};

/// Trait for version repository operations
#[async_trait]
pub trait VersionRepositoryTrait: Send + Sync {
    /// List all versions for an entity
    ///
    /// # Arguments
    /// * `entity_uuid` - UUID of the entity
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_entity_versions(&self, entity_uuid: Uuid) -> Result<Vec<EntityVersionMeta>>;

    /// Get a specific version of an entity
    ///
    /// # Arguments
    /// * `entity_uuid` - UUID of the entity
    /// * `version_number` - Version number to retrieve
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_entity_version(
        &self,
        entity_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<EntityVersionPayload>>;

    /// Insert a version snapshot for an entity
    ///
    /// # Arguments
    /// * `entity_uuid` - UUID of the entity
    /// * `entity_type` - Type of the entity
    /// * `version_number` - Version number
    /// * `data` - Serialized entity data
    /// * `created_by` - UUID of the user creating the snapshot
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn insert_snapshot(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
        version_number: i32,
        data: serde_json::Value,
        created_by: Option<Uuid>,
    ) -> Result<()>;

    /// Prune versions older than specified days
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

    /// Prune versions keeping only the latest N per entity
    ///
    /// # Arguments
    /// * `keep` - Number of latest versions to keep per entity
    ///
    /// # Returns
    /// Number of versions deleted
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn prune_keep_latest_per_entity(&self, keep: i32) -> Result<u64>;

    /// Get current entity metadata from `entities_registry` with resolved creator name
    ///
    /// # Arguments
    /// * `entity_uuid` - UUID of the entity
    ///
    /// # Returns
    /// Tuple of (`version`, `updated_at`, `updated_by`, `updated_by_name`)
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_current_entity_metadata(
        &self,
        entity_uuid: Uuid,
    ) -> Result<Option<(i32, OffsetDateTime, Option<Uuid>, Option<String>)>>;

    /// Get current entity data as JSON from the entity view
    ///
    /// # Arguments
    /// * `entity_uuid` - UUID of the entity
    /// * `entity_type` - Type of the entity
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_current_entity_data(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
    ) -> Result<Option<serde_json::Value>>;
}

