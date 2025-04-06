use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;

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
    pub created_at: DateTime<Utc>,
}

impl VersionedData {
    /// Create a new versioned data snapshot
    pub fn new(entity_uuid: Uuid, version_number: i32, data: serde_json::Value) -> Self {
        Self {
            entity_uuid,
            version_number,
            data,
            created_at: Utc::now(),
        }
    }
    
    /// Try to deserialize this version into a specific entity type
    pub fn deserialize<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.data.clone())
    }
} 