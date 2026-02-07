#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Shared test helpers for `parent_uuid` tests.

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::schema::Schema;
use r_data_core_core::field::options::FieldValidation;
use r_data_core_core::field::types::FieldType;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::FieldDefinition;
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// Create an Instance entity definition with `key_id` field
#[must_use]
pub fn create_instance_definition(entity_type: &str) -> EntityDefinition {
    let mut schema_properties = HashMap::new();
    schema_properties.insert(
        "entity_type".to_string(),
        serde_json::Value::String(entity_type.to_string()),
    );

    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: "Instance".to_string(),
        description: Some("Test instance entity".to_string()),
        group_name: None,
        allow_children: true,
        icon: None,
        fields: vec![FieldDefinition {
            name: "key_id".to_string(),
            display_name: "Key ID".to_string(),
            field_type: FieldType::String,
            description: None,
            required: false,
            indexed: true,
            filterable: true,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        schema: Schema::new(schema_properties),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: true,
        version: 1,
    }
}

/// Create a `StatisticSubmission` entity definition
#[must_use]
pub fn create_submission_definition(entity_type: &str) -> EntityDefinition {
    let mut schema_properties = HashMap::new();
    schema_properties.insert(
        "entity_type".to_string(),
        serde_json::Value::String(entity_type.to_string()),
    );

    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: "Statistic Submission".to_string(),
        description: Some("Test submission entity".to_string()),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "submission_id".to_string(),
                display_name: "Submission ID".to_string(),
                field_type: FieldType::String,
                description: None,
                required: true,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "license_key_id".to_string(),
                display_name: "License Key ID".to_string(),
                field_type: FieldType::String,
                description: None,
                required: false,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: Schema::new(schema_properties),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: true,
        version: 1,
    }
}
