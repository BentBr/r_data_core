#![allow(clippy::unwrap_used)]

use serde_json::json;

use crate::domain::dynamic_entity::entity::DynamicEntity;

use super::create_test_entity;

#[test]
fn test_serialize_deserialize_roundtrip_preserves_entity_type() {
    let entity = create_test_entity();
    let serialized = serde_json::to_string(&entity).unwrap();
    let restored: DynamicEntity = serde_json::from_str(&serialized).unwrap();
    assert_eq!(restored.entity_type, entity.entity_type);
}

#[test]
fn test_serialize_deserialize_roundtrip_preserves_custom_field() {
    let mut entity = create_test_entity();
    entity.set("label", "round-trip".to_string()).unwrap();
    let serialized = serde_json::to_string(&entity).unwrap();
    let restored: DynamicEntity = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        restored.field_data.get("label"),
        entity.field_data.get("label")
    );
}

#[test]
fn test_serialize_deserialize_roundtrip_preserves_system_fields() {
    let entity = create_test_entity();
    let serialized = serde_json::to_string(&entity).unwrap();
    let restored: DynamicEntity = serde_json::from_str(&serialized).unwrap();
    assert_eq!(restored.field_data.get("published"), Some(&json!(false)));
    assert_eq!(restored.field_data.get("version"), Some(&json!(1)));
}

#[test]
fn test_definition_is_skipped_in_serialization() {
    let entity = create_test_entity();
    let serialized = serde_json::to_string(&entity).unwrap();
    // The definition field is #[serde(skip)] — it must not appear in the JSON
    assert!(!serialized.contains("\"definition\""));
}

#[test]
fn test_clone_is_independent() {
    let mut entity = create_test_entity();
    let mut clone = entity.clone();
    clone.set("extra", "cloned".to_string()).unwrap();
    assert!(!entity.field_data.contains_key("extra"));
    entity.set("original_only", "yes".to_string()).unwrap();
    assert!(!clone.field_data.contains_key("original_only"));
}
