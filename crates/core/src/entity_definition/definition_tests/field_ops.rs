use super::create_test_entity_definition;
use crate::entity_definition::definition::*;
use crate::field::{FieldDefinition, FieldType};

// ── get_field (hit and miss) ──────────────────────────────────────────────────

#[test]
fn test_get_field_returns_some_when_field_exists() {
    let def = create_test_entity_definition();
    let field = def.get_field("name");
    assert!(field.is_some());
    assert_eq!(field.unwrap().name, "name");
}

#[test]
fn test_get_field_returns_none_when_field_missing() {
    let def = create_test_entity_definition();
    assert!(def.get_field("nonexistent").is_none());
}

// ── get_fields ────────────────────────────────────────────────────────────────

#[test]
fn test_get_fields_returns_all_fields() {
    let def = create_test_entity_definition();
    assert_eq!(def.get_fields().len(), 1);
    assert_eq!(def.get_fields()[0].name, "name");
}

#[test]
fn test_get_fields_empty_on_default() {
    let def = EntityDefinition::default();
    assert!(def.get_fields().is_empty());
}

// ── add_field ─────────────────────────────────────────────────────────────────

#[test]
fn test_add_field_succeeds_for_new_name() {
    let mut def = EntityDefinition::default();
    let field = FieldDefinition::new("age".to_string(), "Age".to_string(), FieldType::Integer);
    assert!(def.add_field(field).is_ok());
    assert_eq!(def.fields.len(), 1);
}

#[test]
fn test_add_field_errors_on_duplicate_name() {
    let mut def = create_test_entity_definition();
    let dup = FieldDefinition::new("name".to_string(), "Name2".to_string(), FieldType::String);
    let result = def.add_field(dup);
    assert!(result.is_err());
    assert_eq!(def.fields.len(), 1);
}

// ── update_field ──────────────────────────────────────────────────────────────

#[test]
fn test_update_field_succeeds_when_field_exists() {
    let mut def = create_test_entity_definition();
    let mut updated =
        FieldDefinition::new("name".to_string(), "Full Name".to_string(), FieldType::Text);
    updated.required = true;
    assert!(def.update_field(updated).is_ok());
    assert_eq!(def.fields[0].display_name, "Full Name");
    assert!(def.fields[0].required);
}

#[test]
fn test_update_field_errors_when_field_not_found() {
    let mut def = create_test_entity_definition();
    let ghost = FieldDefinition::new("ghost".to_string(), "Ghost".to_string(), FieldType::String);
    assert!(def.update_field(ghost).is_err());
}

// ── remove_field ──────────────────────────────────────────────────────────────

#[test]
fn test_remove_field_succeeds_and_decrements_count() {
    let mut def = create_test_entity_definition();
    assert_eq!(def.fields.len(), 1);
    assert!(def.remove_field("name").is_ok());
    assert!(def.fields.is_empty());
}

#[test]
fn test_remove_field_errors_when_field_not_found() {
    let mut def = create_test_entity_definition();
    assert!(def.remove_field("does_not_exist").is_err());
    assert_eq!(def.fields.len(), 1);
}
