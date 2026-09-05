use super::*;
use crate::entity_definition::definition::EntityDefinition;

mod validate_entity_tests {
    use super::*;

    fn entity_def_with_required_string() -> EntityDefinition {
        EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![create_test_field(FieldType::String, true)],
            published: true,
            ..EntityDefinition::default()
        }
    }

    #[test]
    fn test_missing_entity_type_field() {
        let ed = entity_def_with_required_string();
        assert!(validate_entity(&json!({"field_data":{}}), &ed)
            .unwrap_err()
            .to_string()
            .contains("entity_type field"));
    }

    #[test]
    fn test_entity_type_mismatch() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"wrong","field_data":{}});
        assert!(validate_entity(&e, &ed)
            .unwrap_err()
            .to_string()
            .contains("does not match entity definition type"));
    }

    #[test]
    fn test_missing_field_data() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product"});
        assert!(validate_entity(&e, &ed)
            .unwrap_err()
            .to_string()
            .contains("field_data object"));
    }

    #[test]
    fn test_required_field_missing_creates_violation() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product","field_data":{}});
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "test_field");
        assert!(violations[0].message.contains("required"));
    }

    #[test]
    fn test_unknown_field_creates_violation() {
        let ed = EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![],
            published: true,
            ..EntityDefinition::default()
        };
        let e = json!({"entity_type":"product","field_data":{"unknown_field":"val"}});
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("not defined"));
    }

    #[test]
    fn test_system_fields_skipped() {
        let ed = EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![],
            published: true,
            ..EntityDefinition::default()
        };
        let e = json!({
            "entity_type": "product",
            "field_data": {
                "uuid": "550e8400-e29b-41d4-a716-446655440000",
                "entity_key": "k",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "published": true,
                "version": 1,
                "parent_uuid": null
            }
        });
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert!(violations.is_empty(), "system fields should be skipped");
    }

    #[test]
    fn test_validate_entity_formats_violation_message() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product","field_data":{}});
        let msg = validate_entity(&e, &ed).unwrap_err().to_string();
        assert!(msg.contains("Validation failed with the following errors"));
    }
}

mod validate_parent_path_tests {
    use super::*;

    #[test]
    fn test_no_parent_no_violations() {
        assert!(validate_parent_path_consistency(None, None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_empty_parent_uuid_no_violations() {
        assert!(
            validate_parent_path_consistency(Some(String::new()), None, None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_parent_without_expected_path_no_violations() {
        assert!(validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&"some/path".to_string()),
            None
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn test_path_matches_expected_no_violations() {
        let path = "parent/child".to_string();
        assert!(validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&path),
            Some(&path)
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn test_path_mismatch_creates_violation() {
        let expected = "parent/child".to_string();
        let actual = "wrong/path".to_string();
        let violations = validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&actual),
            Some(&expected),
        )
        .unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "path");
        assert!(violations[0].message.contains("Expected"));
    }

    #[test]
    fn test_missing_path_with_expected_creates_violation() {
        let expected = "parent/child".to_string();
        let violations =
            validate_parent_path_consistency(Some("uuid".to_string()), None, Some(&expected))
                .unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("required when parent_uuid"));
    }
}
