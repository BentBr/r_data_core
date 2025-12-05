#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Schema for dynamic entity serialization
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DynamicEntityResponse {
    pub entity_type: String,
    pub field_data: HashMap<String, Value>,
}

/// Response for entity creation/update
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EntityResponse {
    pub uuid: Uuid,
    pub entity_type: String,
}

// Note: From<DynamicEntity> implementation must be in the main crate
// since DynamicEntity is defined in r_data_core_core
