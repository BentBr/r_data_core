// Basic versioning module - to be expanded in future
use crate::error::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

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
    fn create_version(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
        data: &serde_json::Value,
        user_uuid: Option<Uuid>,
        comment: Option<&str>,
    ) -> Result<Version>;

    /// Get a specific version
    fn get_version(&self, entity_uuid: Uuid, version: i32) -> Result<Version>;

    /// Get all versions for an entity
    fn get_versions(&self, entity_uuid: Uuid) -> Result<Vec<Version>>;

    /// Get the latest version for an entity
    fn get_latest_version(&self, entity_uuid: Uuid) -> Result<Version>;

    /// Revert to a specific version
    fn revert_to_version(
        &self,
        entity_uuid: Uuid,
        version: i32,
        user_uuid: Option<Uuid>,
    ) -> Result<Version>;
}
