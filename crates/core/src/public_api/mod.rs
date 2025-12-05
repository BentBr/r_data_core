#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use utoipa::ToSchema;

/// Entity type information for dynamic entities
///
/// Represents an entity definition that can be used to create dynamic entity instances.
/// Entity types are defined by entity definitions.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct EntityTypeInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub entity_count: i64,
    pub field_count: i32,
}

/// Query parameters for entity filtering
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct EntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

/// Kind of browse node
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowseKind {
    Folder,
    File,
}

/// Node returned when browsing dynamic entities by virtual path
///
/// Represents either a folder (virtual path segment) or a file (dynamic entity instance)
/// in the hierarchical structure of dynamic entities.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct BrowseNode {
    /// "folder" or "file"
    pub kind: BrowseKind,
    /// Item name (folder segment or file name)
    pub name: String,
    /// Full path for this item
    pub path: String,
    /// Present for files or folder-entities that exist as dynamic entity instances
    pub entity_uuid: Option<Uuid>,
    /// Entity type (from entity definition) if this is a dynamic entity instance
    pub entity_type: Option<String>,
    /// Whether this node has children (for folders or entities with child entities)
    pub has_children: Option<bool>,
}

/// Advanced query for dynamic entities with complex filtering
///
/// Used to query dynamic entity instances with advanced filtering capabilities.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdvancedEntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}
