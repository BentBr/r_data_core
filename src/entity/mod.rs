mod abstract_entity;
pub mod admin_user;
mod notification;
mod permission_scheme;
pub mod refresh_token;
mod version;
pub mod version_repository;
mod workflow;
// mod workflow_definition;
// mod workflow_task;

pub mod dynamic_entity;
pub mod entity_definition;
pub mod field;

pub use abstract_entity::AbstractRDataEntity;
pub use admin_user::AdminUser;
pub use dynamic_entity::DynamicEntity;
pub use version::VersionedData;
//pub use value;
pub use entity_definition::EntityDefinition;

use crate::error::Result;
use std::collections::HashMap;

/// Trait for any entity that can be serialized to/from JSON for custom fields
pub trait DynamicFields {
    fn get_field(&self, name: &str) -> Option<serde_json::Value>;
    fn set_field(&mut self, name: &str, value: serde_json::Value) -> Result<()>;
    fn get_all_fields(&self) -> HashMap<String, serde_json::Value>;
    fn validate(&self, entity_def: &EntityDefinition) -> Result<()>;
}
