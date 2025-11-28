#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Entity type information returned to clients
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EntityTypeInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub entity_count: i64,
    pub field_count: i32,
}

/// Query parameters for entity filtering
#[derive(Debug, Deserialize, ToSchema)]
pub struct EntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

/// Kind of browse node
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowseKind {
    Folder,
    File,
}

/// Node returned when browsing entities by virtual path
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub struct BrowseNode {
    /// "folder" or "file"
    pub kind: BrowseKind,
    /// Item name (folder segment or file name)
    pub name: String,
    /// Full path for this item
    pub path: String,
    /// Present for files or folder-entities that exist as entities
    pub entity_uuid: Option<Uuid>,
    /// Type of the entity if present
    pub entity_type: Option<String>,
    /// Whether the folder has children (only meaningful when kind = folder)
    pub has_children: Option<bool>,
}

/// Version metadata for entity versions
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionMeta {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

/// Version payload containing the actual entity data
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionPayload {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}

/// Request body for querying entities
#[derive(Debug, Deserialize, ToSchema)]
pub struct EntityQueryRequest {
    /// Entity type to query
    pub entity_type: String,
    /// Filter by parent UUID
    pub parent_uuid: Option<Uuid>,
    /// Filter by exact path
    pub path: Option<String>,
    /// Maximum number of results (default: 20, max: 100)
    pub limit: Option<i64>,
    /// Number of results to skip (default: 0)
    pub offset: Option<i64>,
}
