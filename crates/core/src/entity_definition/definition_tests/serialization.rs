use super::create_test_entity_definition;
use crate::entity_definition::definition::*;
use crate::field::options::FieldValidation;
use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use uuid::Uuid;

#[test]
fn test_entity_definition_serializes_and_deserializes() {
    let def = create_test_entity_definition();
    let json = serde_json::to_string(&def).unwrap();
    let restored: EntityDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.entity_type, def.entity_type);
    assert_eq!(restored.display_name, def.display_name);
    assert_eq!(restored.published, def.published);
    assert_eq!(restored.version, def.version);
    assert_eq!(restored.fields.len(), def.fields.len());
}

#[test]
fn test_entity_definition_params_fields_roundtrip_via_from_params() {
    let field = FieldDefinition {
        name: "title".to_string(),
        display_name: "Title".to_string(),
        field_type: FieldType::String,
        description: None,
        required: true,
        indexed: true,
        filterable: false,
        unique: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
    };
    let params = EntityDefinitionParams {
        entity_type: "article".to_string(),
        display_name: "Article".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![field],
        created_by: Uuid::nil(),
    };
    let def = EntityDefinition::from_params(params);
    assert_eq!(def.get_fields().len(), 1);
    assert!(def.get_field("title").is_some());
    assert!(def.get_field("missing").is_none());
}

#[test]
fn test_serialized_json_contains_published_and_version() {
    let def = create_test_entity_definition();
    let json = serde_json::to_string(&def).unwrap();
    assert!(json.contains("\"published\":false"));
    assert!(json.contains("\"version\":1"));
}
