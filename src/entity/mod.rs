mod abstract_entity;
pub mod admin_user;
mod notification;
mod permission_scheme;
mod version;
mod workflow;
// mod workflow_definition;
// mod workflow_task;

pub mod class;
pub mod dynamic_entity;
pub mod field;

pub use abstract_entity::AbstractRDataEntity;
pub use admin_user::AdminUser;
pub use notification::Notification;
pub use permission_scheme::PermissionScheme;
pub use version::VersionedData;
pub use workflow::WorkflowEntity;
// pub use workflow_definition::{WorkflowDefinition, WorkflowState, WorkflowTrigger, WorkflowAction};
// pub use workflow_task::{WorkflowTask, TaskStatus};
pub use dynamic_entity::DynamicEntity;
//pub use value;
pub use class::ClassDefinition;
pub use field::{
    FieldDefinition, FieldType, FieldValidation, OptionsSource, SelectOption, UiSettings,
};

use crate::error::Error;
use std::collections::HashMap;

/// Trait for any entity that can be serialized to/from JSON for custom fields
pub trait DynamicFields {
    fn get_field(&self, name: &str) -> Option<serde_json::Value>;
    fn set_field(&mut self, name: &str, value: serde_json::Value) -> Result<(), Error>;
    fn get_all_fields(&self) -> HashMap<String, serde_json::Value>;
    fn validate(&self, class_def: &ClassDefinition) -> Result<(), Error>;
}
