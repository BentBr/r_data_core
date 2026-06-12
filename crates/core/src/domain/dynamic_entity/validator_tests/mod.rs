#![allow(clippy::unwrap_used)]

use super::validator::*;
use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use serde_json::json;

mod datetime_types;
mod entity_validation;
mod free_fn;
mod json_types;
mod scalar_types;
mod select_types;
mod violations_detail;

pub(super) fn create_test_field(field_type: FieldType, required: bool) -> FieldDefinition {
    FieldDefinition {
        name: "test_field".to_string(),
        display_name: "Test Field".to_string(),
        field_type,
        required,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}

pub(super) fn create_named_field(
    name: &str,
    display_name: &str,
    field_type: FieldType,
) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        field_type,
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}
