use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Advanced query parameters for complex entity filtering
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdvancedEntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
    pub include_related: Option<bool>,
    pub fields: Option<Vec<String>>,
}
