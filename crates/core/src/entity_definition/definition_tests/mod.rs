#![allow(clippy::unwrap_used)]

mod constructors;
mod field_ops;
mod serialization;
mod sql_gen;
mod validation;

use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use uuid::Uuid;

use crate::entity_definition::definition::*;
use crate::entity_definition::schema::Schema;

pub(super) fn create_test_entity_definition() -> EntityDefinition {
    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: "test".to_string(),
        display_name: "Test Entity".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            field_type: FieldType::String,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: crate::field::options::FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: std::collections::HashMap::new(),
        }],
        schema: Schema::default(),
        created_at: time::OffsetDateTime::now_utc(),
        updated_at: time::OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: false,
        version: 1,
    }
}
