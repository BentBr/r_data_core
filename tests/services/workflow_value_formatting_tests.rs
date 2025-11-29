#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::field::definition::FieldDefinition;
use r_data_core_core::field::types::FieldType;
use r_data_core_services::workflow::value_formatting::{
    build_normalized_field_data, cast_field_value, normalize_field_data_by_type,
    process_reserved_field, PROTECTED_FIELDS, RESERVED_FIELDS,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn test_normalize_field_data_by_type() {
    let mut field_data = HashMap::new();
    field_data.insert("is_active".to_string(), json!("true"));
    field_data.insert("age".to_string(), json!("25"));
    field_data.insert("price".to_string(), json!("19.99"));
    field_data.insert("name".to_string(), json!("Test"));

    let fields = vec![
        FieldDefinition::new(
            "is_active".to_string(),
            "Is Active".to_string(),
            FieldType::Boolean,
        ),
        FieldDefinition::new("age".to_string(), "Age".to_string(), FieldType::Integer),
        FieldDefinition::new("price".to_string(), "Price".to_string(), FieldType::Float),
        FieldDefinition::new("name".to_string(), "Name".to_string(), FieldType::String),
    ];

    let def = EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: "test".to_string(),
        display_name: "Test".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields,
        schema: Default::default(),
        created_at: time::OffsetDateTime::now_utc(),
        updated_at: time::OffsetDateTime::now_utc(),
        created_by: Uuid::now_v7(),
        updated_by: None,
        version: 1,
        published: false,
    };

    normalize_field_data_by_type(&mut field_data, &def);

    assert_eq!(field_data.get("is_active"), Some(&json!(true)));
    assert_eq!(field_data.get("age"), Some(&json!(25)));
    assert_eq!(field_data.get("price"), Some(&json!(19.99)));
    assert_eq!(field_data.get("name"), Some(&json!("Test")));
}

#[test]
fn test_build_normalized_field_data_complex() {
    let mut field_data = HashMap::new();
    field_data.insert("name".to_string(), json!("Test Entity"));
    field_data.insert("newsletter".to_string(), json!("false"));
    field_data.insert("crm".to_string(), json!("true"));
    field_data.insert("published".to_string(), json!("1"));
    field_data.insert("created_at".to_string(), json!("2024-01-01"));
    field_data.insert(
        "uuid".to_string(),
        json!("123e4567-e89b-12d3-a456-426614174000"),
    );

    let def = EntityDefinition::new(
        "customer".to_string(),
        "Customer".to_string(),
        None,
        None,
        false,
        None,
        vec![
            FieldDefinition::new(
                "newsletter".to_string(),
                "Newsletter".to_string(),
                FieldType::Boolean,
            ),
            FieldDefinition::new("crm".to_string(), "CRM".to_string(), FieldType::Boolean),
        ],
        Uuid::now_v7(),
    );

    let normalized = build_normalized_field_data(field_data, &def);

    // Regular fields should be preserved
    assert_eq!(normalized.get("name"), Some(&json!("Test Entity")));

    // Protected fields should be removed
    assert!(!normalized.contains_key("created_at"));

    // Reserved fields should be preserved
    assert!(normalized.contains_key("uuid"));

    // Published should be coerced to boolean
    assert_eq!(normalized.get("published"), Some(&json!(true)));
}

#[test]
fn test_reserved_fields_constant() {
    assert!(RESERVED_FIELDS.contains(&"uuid"));
    assert!(RESERVED_FIELDS.contains(&"path"));
    assert!(RESERVED_FIELDS.contains(&"published"));
    assert!(RESERVED_FIELDS.len() == 10);
}

#[test]
fn test_protected_fields_constant() {
    assert!(PROTECTED_FIELDS.contains(&"created_at"));
    assert!(PROTECTED_FIELDS.contains(&"created_by"));
    assert!(PROTECTED_FIELDS.len() == 2);
}

#[test]
fn test_process_reserved_field_entity_key() {
    let mut normalized = HashMap::new();
    process_reserved_field("entity_key", json!("test-key-123"), &mut normalized);
    assert_eq!(normalized.get("entity_key"), Some(&json!("test-key-123")));
}

#[test]
fn test_process_reserved_field_path() {
    let mut normalized = HashMap::new();
    process_reserved_field("path", json!("/custom/path"), &mut normalized);
    assert_eq!(normalized.get("path"), Some(&json!("/custom/path")));
}

#[test]
fn test_cast_field_value_string_type() {
    // String types should remain unchanged
    let result = cast_field_value(json!("test"), &FieldType::String);
    assert_eq!(result, json!("test"));
}

#[test]
fn test_cast_field_value_date_type() {
    // Date types should remain unchanged
    let result = cast_field_value(json!("2024-01-01"), &FieldType::Date);
    assert_eq!(result, json!("2024-01-01"));
}
