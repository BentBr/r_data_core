use crate::entity_definition::definition::*;
use crate::field::{FieldDefinition, FieldType};

fn valid_def() -> EntityDefinition {
    EntityDefinition {
        entity_type: "my_entity".to_string(),
        display_name: "My Entity".to_string(),
        ..EntityDefinition::default()
    }
}

#[test]
fn test_validate_passes_for_valid_definition() {
    assert!(valid_def().validate().is_ok());
}

#[test]
fn test_validate_fails_when_entity_type_empty() {
    let mut d = valid_def();
    d.entity_type = String::new();
    assert!(d.validate().is_err());
}

#[test]
fn test_validate_fails_when_entity_type_has_spaces() {
    let mut d = valid_def();
    d.entity_type = "my entity".to_string();
    assert!(d.validate().is_err());
}

#[test]
fn test_validate_fails_when_entity_type_has_hyphen() {
    let mut d = valid_def();
    d.entity_type = "my-entity".to_string();
    assert!(d.validate().is_err());
}

#[test]
fn test_validate_fails_when_display_name_empty() {
    let mut d = valid_def();
    d.display_name = String::new();
    assert!(d.validate().is_err());
}

#[test]
fn test_validate_fails_on_duplicate_field_names() {
    let mut d = valid_def();
    let f1 = FieldDefinition::new("score".to_string(), "Score".to_string(), FieldType::Integer);
    let f2 = FieldDefinition::new(
        "score".to_string(),
        "Score2".to_string(),
        FieldType::Integer,
    );
    d.fields.push(f1);
    d.fields.push(f2);
    assert!(d.validate().is_err());
}

#[test]
fn test_validate_passes_with_multiple_distinct_fields() {
    let mut d = valid_def();
    d.fields.push(FieldDefinition::new(
        "a".to_string(),
        "A".to_string(),
        FieldType::String,
    ));
    d.fields.push(FieldDefinition::new(
        "b".to_string(),
        "B".to_string(),
        FieldType::Integer,
    ));
    assert!(d.validate().is_ok());
}

#[test]
fn test_validate_accepts_underscores_in_entity_type() {
    let mut d = valid_def();
    d.entity_type = "my_entity_type_123".to_string();
    assert!(d.validate().is_ok());
}
