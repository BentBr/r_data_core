#![allow(clippy::unwrap_used)]

use crate::field::definition::FieldDefinition;
use crate::field::options::FieldValidation;
use crate::field::ui::UiSettings;
use crate::field::FieldType;
use serde_json::json;

pub(super) fn create_field_definition(name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: name.to_string(),
        field_type,
        description: None,
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
    }
}

mod boolean_required;
mod date_uuid;
mod field_definition;
mod json_object_array;
mod numeric_fields;
mod select_fields;
mod string_field;
