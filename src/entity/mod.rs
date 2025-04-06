mod abstract_entity;
pub mod admin_user;
mod permission_scheme;
mod notification;
mod workflow;
mod version;
// mod workflow_definition;
// mod workflow_task;

pub mod field;
pub mod class;
pub mod dynamic_entity;

pub use abstract_entity::AbstractRDataEntity;
pub use admin_user::AdminUser;
pub use permission_scheme::PermissionScheme;
pub use notification::Notification;
pub use workflow::WorkflowEntity;
pub use version::VersionedData;
// pub use workflow_definition::{WorkflowDefinition, WorkflowState, WorkflowTrigger, WorkflowAction};
// pub use workflow_task::{WorkflowTask, TaskStatus};
pub use dynamic_entity::DynamicEntity;
//pub use value;
pub use field::{FieldType, FieldDefinition, FieldValidation, UiSettings, OptionsSource, SelectOption};
pub use class::ClassDefinition;

use crate::error::Result;
use std::collections::HashMap;

/// Trait for any entity that can be serialized to/from JSON for custom fields
pub trait DynamicFields {
    fn get_field(&self, name: &str) -> Option<serde_json::Value>;
    fn set_field(&mut self, name: &str, value: serde_json::Value) -> Result<()>;
    fn get_all_fields(&self) -> HashMap<String, serde_json::Value>;
} 