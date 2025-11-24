use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod purger_trait;

/// Represents a versioned snapshot of an entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VersionedData {
    /// UUID of the original entity
    pub entity_uuid: Uuid,

    /// Version number of this snapshot
    pub version_number: i32,

    /// Serialized entity data at this version
    pub data: serde_json::Value,

    /// When this version was created
    pub created_at: OffsetDateTime,
}

impl VersionedData {
    /// Create a new versioned data snapshot
    #[must_use]
    pub fn new(entity_uuid: Uuid, version_number: i32, data: serde_json::Value) -> Self {
        Self {
            entity_uuid,
            version_number,
            data,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    /// Try to deserialize this version into a specific entity type
    ///
    /// # Errors
    /// Returns a `serde_json::Error` if deserialization fails
    pub fn deserialize<T>(&self) -> std::result::Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.data.clone())
    }
}

/// Version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Version number
    pub version: i32,
    /// Entity UUID
    pub entity_uuid: Uuid,
    /// Entity type
    pub entity_type: String,
    /// Data at this version
    pub data: serde_json::Value,
    /// User who created this version
    pub created_by: Uuid,
    /// When this version was created
    pub created_at: OffsetDateTime,
    /// Comment for this version
    pub comment: Option<String>,
}

/// Version manager trait
pub trait VersionManager {
    /// Create a new version
    ///
    /// # Errors
    /// Returns an error if version creation fails
    fn create_version(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
        data: &serde_json::Value,
        user_uuid: Option<Uuid>,
        comment: Option<&str>,
    ) -> Result<Version>;

    /// Get a specific version
    ///
    /// # Errors
    /// Returns an error if the version is not found or retrieval fails
    fn get_version(&self, entity_uuid: Uuid, version: i32) -> Result<Version>;

    /// Get all versions for an entity
    ///
    /// # Errors
    /// Returns an error if version retrieval fails
    fn get_versions(&self, entity_uuid: Uuid) -> Result<Vec<Version>>;

    /// Get the latest version for an entity
    ///
    /// # Errors
    /// Returns an error if no versions exist or retrieval fails
    fn get_latest_version(&self, entity_uuid: Uuid) -> Result<Version>;

    /// Revert to a specific version
    ///
    /// # Errors
    /// Returns an error if reverting fails
    fn revert_to_version(
        &self,
        entity_uuid: Uuid,
        version: i32,
        user_uuid: Option<Uuid>,
    ) -> Result<Version>;
}
