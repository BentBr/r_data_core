#![allow(clippy::unwrap_used)]

use serde_json::json;

use crate::domain::DynamicFields;

use super::create_test_entity;

#[test]
fn test_get_field_returns_some_for_existing() {
    let entity = create_test_entity();
    assert!(entity.get_field("path").is_some());
}

#[test]
fn test_get_field_returns_none_for_missing() {
    let entity = create_test_entity();
    assert!(entity.get_field("no_such_field").is_none());
}

#[test]
fn test_set_field_stores_value() {
    let mut entity = create_test_entity();
    entity.set_field("x", json!(7)).unwrap();
    assert_eq!(entity.get_field("x"), Some(json!(7)));
}

#[test]
fn test_set_field_overwrites_existing() {
    let mut entity = create_test_entity();
    entity.set_field("x", json!(1)).unwrap();
    entity.set_field("x", json!(2)).unwrap();
    assert_eq!(entity.get_field("x"), Some(json!(2)));
}

#[test]
fn test_get_all_fields_contains_system_fields() {
    let entity = create_test_entity();
    let fields = entity.get_all_fields();
    assert!(fields.contains_key("path"));
    assert!(fields.contains_key("published"));
    assert!(fields.contains_key("version"));
}

#[test]
fn test_get_all_fields_is_independent_clone() {
    let entity = create_test_entity();
    let mut fields = entity.get_all_fields();
    fields.insert("mutated".to_string(), json!(true));
    // The returned map is a copy: it has the mutation...
    assert_eq!(fields.get("mutated"), Some(&json!(true)));
    // ...but the original entity is unaffected.
    assert!(entity.get_field("mutated").is_none());
}

#[test]
fn test_trait_validate_passes_with_no_schema() {
    let entity = create_test_entity();
    // Disambiguate: DynamicEntity has an inherent validate() (0 args);
    // DynamicFields::validate takes schema_properties.
    assert!(DynamicFields::validate(&entity, None).is_ok());
}

#[test]
fn test_trait_validate_passes_when_required_schema_field_present() {
    let mut entity = create_test_entity();
    entity.set_field("title", json!("hello")).unwrap();
    let schema = json!({
        "title": { "required": true }
    });
    assert!(DynamicFields::validate(&entity, Some(&schema)).is_ok());
}

#[test]
fn test_trait_validate_fails_when_required_schema_field_missing() {
    let entity = create_test_entity();
    let schema = json!({
        "missing_field": { "required": true }
    });
    let result = DynamicFields::validate(&entity, Some(&schema));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing_field"));
}

#[test]
fn test_trait_validate_ignores_non_required_schema_fields() {
    let entity = create_test_entity();
    let schema = json!({
        "optional_field": { "required": false }
    });
    assert!(DynamicFields::validate(&entity, Some(&schema)).is_ok());
}
