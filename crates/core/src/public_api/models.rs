#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod models;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Entity type information returned to clients
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityTypeInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub entity_count: i64,
    pub field_count: i32,
}

/// Query parameters for entity filtering
#[derive(Debug, Deserialize, Clone)]
pub struct EntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

/// Kind of browse node
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowseKind {
    Folder,
    File,
}

/// Node returned when browsing entities by virtual path
#[derive(Debug, Serialize, Deserialize, Clone)]
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
    /// Whether this node has children (for folders or entities with child entities)
    pub has_children: Option<bool>,
}

/// Advanced entity query with complex filtering
#[derive(Debug, Deserialize, Clone)]
pub struct AdvancedEntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

