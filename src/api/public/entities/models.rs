use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

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

/// Helper function to convert a JSON value
pub fn convert_value(value: &serde_json::Value) -> serde_json::Value {
    value.clone()
}
