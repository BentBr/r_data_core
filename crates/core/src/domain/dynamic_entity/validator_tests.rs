#![allow(clippy::unwrap_used)]

use super::validator::*;
use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use serde_json::json;

fn create_test_field(field_type: FieldType, required: bool) -> FieldDefinition {
    FieldDefinition {
        name: "test_field".to_string(),
        display_name: "Test Field".to_string(),
        field_type,
        required,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}

fn create_named_field(name: &str, display_name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        field_type,
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}

mod validate_json_tests {
    use super::*;

    #[test]
    fn test_validate_json_accepts_object() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!({"key": "value", "nested": {"inner": 1}});
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_string() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!("a plain string value");
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_array() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!([1, 2, 3]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_array_of_objects() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!([
            {"entity_type": "Customer", "count": 100},
            {"entity_type": "Order", "count": 500}
        ]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_number() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!(123);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_boolean() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!(true);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_allows_null_when_not_required() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!(null);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_rejects_null_when_required() {
        let field_def = create_test_field(FieldType::Json, true);
        let value = json!(null);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("is required"));
    }

    #[test]
    fn test_validate_json_accepts_complex_nested_object() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!({
            "order_items": {
                "0": "product1",
                "1": "product2"
            },
            "metadata": {
                "count": 2,
                "tags": ["tag1", "tag2"]
            }
        });
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_empty_object() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!({});
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_accepts_empty_array() {
        let field_def = create_test_field(FieldType::Json, false);
        let value = json!([]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }
}

mod validate_object_tests {
    use super::*;

    #[test]
    fn test_validate_object_accepts_object() {
        let field_def = create_test_field(FieldType::Object, false);
        let value = json!({"key": "value"});
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_object_rejects_string() {
        let field_def = create_test_field(FieldType::Object, false);
        let value = json!("not an object");
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("must be an object"));
    }

    #[test]
    fn test_validate_object_rejects_array() {
        let field_def = create_test_field(FieldType::Object, false);
        let value = json!([1, 2, 3]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_err());
    }
}

mod validate_array_tests {
    use super::*;

    #[test]
    fn test_validate_array_accepts_array() {
        let field_def = create_test_field(FieldType::Array, false);
        let value = json!([1, 2, 3]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_array_accepts_array_of_objects() {
        let field_def = create_test_field(FieldType::Array, false);
        let value = json!([{"id": 1}, {"id": 2}]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_array_rejects_object() {
        let field_def = create_test_field(FieldType::Array, false);
        let value = json!({"key": "value"});
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn test_validate_array_rejects_string() {
        let field_def = create_test_field(FieldType::Array, false);
        let value = json!("[1, 2, 3]"); // String representation of array
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_array_accepts_empty_array() {
        let field_def = create_test_field(FieldType::Array, false);
        let value = json!([]);
        let result = DynamicEntityValidator::validate_field(&field_def, &value);
        assert!(result.is_ok());
    }
}

mod validate_entity_with_violations_tests {
    use super::*;
    use crate::entity_definition::definition::EntityDefinition;

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
        // Json field type now accepts any valid JSON value including strings
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
        // Json field type now accepts any valid JSON value including arrays
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
                "items": "[1, 2, 3]" // String instead of array
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
                "json_content": "valid json string value",  // Json accepts any value
                "items": "not an array"  // Array field requires array
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        // Only items field should have a violation (json_content accepts any value now)
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
                "items": "not an array"  // String instead of array
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);

        // Message should be clean without the redundant "Field 'x'" prefix
        // since the field name is in the separate 'field' property
        let msg = &violations[0].message;
        assert_eq!(msg, "must be an array");
    }
}
