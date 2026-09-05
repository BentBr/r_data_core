use super::*;
use crate::entity_definition::definition::EntityDefinition;

mod validate_entity_with_violations_tests {
    use super::*;

    fn create_test_entity_definition() -> EntityDefinition {
        EntityDefinition {
            entity_type: "test_entity".to_string(),
            display_name: "Test Entity".to_string(),
            fields: vec![
                create_test_field(FieldType::String, false),
                create_named_field("json_content", "JSON Content", FieldType::Json),
                create_named_field("items", "Items", FieldType::Array),
            ],
            published: true,
            ..EntityDefinition::default()
        }
    }

    #[test]
    fn test_json_field_accepts_string_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": "a plain string value"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(
            violations.is_empty(),
            "Json field should accept string values"
        );
    }

    #[test]
    fn test_json_field_accepts_array_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": [{"key": "value"}, {"key": "value2"}]
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(
            violations.is_empty(),
            "Json field should accept array values"
        );
    }

    #[test]
    fn test_violations_for_array_string_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": "[1, 2, 3]"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "items");
        assert!(violations[0].message.contains("must be an array"));
    }

    #[test]
    fn test_no_violations_for_valid_json_object() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": {"key": "value", "count": 5}
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(violations.is_empty());
    }

    #[test]
    fn test_no_violations_for_valid_array() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": [{"id": 1}, {"id": 2}]
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(violations.is_empty());
    }

    #[test]
    fn test_violations_for_invalid_array_field() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": "valid json string value",
                "items": "not an array"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations
            .iter()
            .map(|v| v.field.as_str())
            .any(|x| x == "items"));
    }

    #[test]
    fn test_violation_message_format_for_array() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": "not an array"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);

        let msg = &violations[0].message;
        assert_eq!(msg, "must be an array");
    }
}
