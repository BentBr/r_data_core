#![allow(clippy::implicit_hasher)] // Functions take concrete HashMap; generalizing over BuildHasher is unnecessary here

mod crud;
mod lookup;
mod path_resolution;

pub use crud::{create_entity, create_or_update_entity, update_entity};
pub use lookup::{ensure_audit_fields, find_existing_entity};
pub use path_resolution::{
    get_or_create_entity_by_path, get_or_create_parent_entity, resolve_dynamic_path,
    resolve_entity_path, ResolvedEntityPath,
};

use r_data_core_core::DynamicEntity;
use serde_json::Value;
use uuid::Uuid;

/// Context for entity persistence operations
pub struct PersistenceContext {
    pub entity_type: String,
    pub produced: Value,
    pub path: Option<String>,
    pub run_uuid: Uuid,
    pub update_key: Option<String>,
    pub skip_versioning: bool,
}

/// Result of entity lookup
pub enum EntityLookupResult {
    Found(DynamicEntity),
    NotFound,
}
