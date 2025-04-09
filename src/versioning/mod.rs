// Basic versioning module - to be expanded in future
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Version number
    pub version: i32,
    /// Entity UUID
    pub entity_id: Uuid,
    /// Entity type
    pub entity_type: String,
    /// Data at this version
    pub data: serde_json::Value,
    /// User who created this version
    pub created_by: Option<Uuid>,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Comment for this version
    pub comment: Option<String>,
}

/// Version manager trait
pub trait VersionManager {
    /// Create a new version
    fn create_version(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        data: &serde_json::Value,
        user_id: Option<Uuid>,
        comment: Option<&str>,
    ) -> Result<Version>;

    /// Get a specific version
    fn get_version(&self, entity_id: Uuid, version: i32) -> Result<Version>;

    /// Get all versions for an entity
    fn get_versions(&self, entity_id: Uuid) -> Result<Vec<Version>>;

    /// Get the latest version for an entity
    fn get_latest_version(&self, entity_id: Uuid) -> Result<Version>;

    /// Revert to a specific version
    fn revert_to_version(
        &self,
        entity_id: Uuid,
        version: i32,
        user_id: Option<Uuid>,
    ) -> Result<Version>;
}
