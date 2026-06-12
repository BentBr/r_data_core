use crate::entity_definition::definition::*;
use uuid::Uuid;

#[test]
fn test_default_produces_zero_uuid_and_empty_strings() {
    let def = EntityDefinition::default();
    assert_eq!(def.uuid, Uuid::nil());
    assert!(def.entity_type.is_empty());
    assert!(def.display_name.is_empty());
    assert!(def.fields.is_empty());
    assert!(!def.allow_children);
    assert!(!def.published);
    assert_eq!(def.version, 1);
    assert!(def.description.is_none());
    assert!(def.group_name.is_none());
    assert!(def.icon.is_none());
    assert!(def.updated_by.is_none());
}

#[test]
fn test_from_params_populates_all_fields() {
    let creator = Uuid::now_v7();
    let params = EntityDefinitionParams {
        entity_type: "product".to_string(),
        display_name: "Product".to_string(),
        description: Some("A product entity".to_string()),
        group_name: Some("catalog".to_string()),
        allow_children: true,
        icon: Some("box".to_string()),
        fields: vec![],
        created_by: creator,
    };

    let def = EntityDefinition::from_params(params);

    assert_eq!(def.entity_type, "product");
    assert_eq!(def.display_name, "Product");
    assert_eq!(def.description.as_deref(), Some("A product entity"));
    assert_eq!(def.group_name.as_deref(), Some("catalog"));
    assert!(def.allow_children);
    assert_eq!(def.icon.as_deref(), Some("box"));
    assert_eq!(def.created_by, creator);
    assert!(!def.published);
    assert_eq!(def.version, 1);
    assert!(def.updated_by.is_none());
    assert_ne!(def.uuid, Uuid::nil());
}

#[test]
fn test_from_params_minimal_fields() {
    let params = EntityDefinitionParams {
        entity_type: "simple".to_string(),
        display_name: "Simple".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![],
        created_by: Uuid::nil(),
    };
    let def = EntityDefinition::from_params(params);
    assert!(def.description.is_none());
    assert!(def.group_name.is_none());
    assert!(def.icon.is_none());
    assert!(!def.allow_children);
}
