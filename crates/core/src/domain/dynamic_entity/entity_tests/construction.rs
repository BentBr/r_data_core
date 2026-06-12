#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use serde_json::json;

use crate::domain::dynamic_entity::entity::DynamicEntity;
use crate::entity_definition::definition::EntityDefinition;

use super::create_test_entity;

#[test]
fn test_new_initialises_system_fields() {
    let entity = create_test_entity();
    assert_eq!(entity.entity_type, "test_type");
    assert!(entity.field_data.contains_key("path"));
    assert!(entity.field_data.contains_key("created_at"));
    assert!(entity.field_data.contains_key("updated_at"));
    assert!(entity.field_data.contains_key("published"));
    assert!(entity.field_data.contains_key("version"));
}

#[test]
fn test_new_path_is_lowercased_entity_type() {
    let entity = create_test_entity();
    assert_eq!(entity.field_data.get("path").unwrap(), &json!("/test_type"));
}

#[test]
fn test_new_published_defaults_to_false() {
    let entity = create_test_entity();
    assert_eq!(entity.field_data.get("published").unwrap(), &json!(false));
}

#[test]
fn test_new_version_defaults_to_one() {
    let entity = create_test_entity();
    assert_eq!(entity.field_data.get("version").unwrap(), &json!(1));
}

#[test]
fn test_from_data_stores_supplied_fields() {
    let definition = Arc::new(EntityDefinition::default());
    let mut data = std::collections::HashMap::new();
    data.insert("foo".to_string(), json!("bar"));
    let entity = DynamicEntity::from_data("my_type".to_string(), data, definition);
    assert_eq!(entity.entity_type, "my_type");
    assert_eq!(entity.field_data.get("foo").unwrap(), &json!("bar"));
}

#[test]
fn test_from_data_does_not_add_system_fields() {
    let definition = Arc::new(EntityDefinition::default());
    let entity = DynamicEntity::from_data(
        "my_type".to_string(),
        std::collections::HashMap::new(),
        definition,
    );
    // from_data is raw — no system fields injected
    assert!(!entity.field_data.contains_key("version"));
}

#[test]
fn test_default_entity_type_is_empty() {
    let entity = DynamicEntity::default();
    assert!(entity.entity_type.is_empty());
    assert!(entity.field_data.is_empty());
}

#[test]
fn test_new_uppercased_entity_type_lowercased_in_path() {
    let definition = Arc::new(EntityDefinition::default());
    let entity = DynamicEntity::new("MyEntity".to_string(), definition);
    assert_eq!(entity.field_data.get("path").unwrap(), &json!("/myentity"));
}
