#![allow(clippy::unwrap_used)]

use crate::field::definition::FieldDefinition;
use crate::field::options::FieldValidation;
use crate::field::ui::UiSettings;
use crate::field::FieldType;
use serde_json::json;

fn create_field_definition(name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: name.to_string(),
        field_type,
        description: None,
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
    }
}

mod json_field_validation {
    use super::*;

    #[test]
    fn test_json_field_accepts_array() {
        let field = create_field_definition("cors_origins", FieldType::Json);
        let value = json!(["https://example.com", "https://test.com"]);
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_empty_array() {
        let field = create_field_definition("cors_origins", FieldType::Json);
        let value = json!([]);
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_array_of_objects() {
        let field = create_field_definition("entities_per_definition", FieldType::Json);
        let value = json!([
            {"entity_type": "Customer", "count": 100},
            {"entity_type": "Order", "count": 500}
        ]);
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_object() {
        let field = create_field_definition("metadata", FieldType::Json);
        let value = json!({"key": "value"});
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_string() {
        let field = create_field_definition("data", FieldType::Json);
        let value = json!("a plain string");
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_number() {
        let field = create_field_definition("data", FieldType::Json);
        let value = json!(42);
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_json_field_accepts_boolean() {
        let field = create_field_definition("data", FieldType::Json);
        let value = json!(true);
        assert!(field.validate_value(&value).is_ok());
    }
}

mod object_field_validation {
    use super::*;

    #[test]
    fn test_object_field_accepts_object() {
        let field = create_field_definition("metadata", FieldType::Object);
        let value = json!({"key": "value"});
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_object_field_rejects_array() {
        let field = create_field_definition("metadata", FieldType::Object);
        let value = json!([1, 2, 3]);
        let result = field.validate_value(&value);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be an object"));
    }

    #[test]
    fn test_object_field_rejects_string() {
        let field = create_field_definition("metadata", FieldType::Object);
        let value = json!("not an object");
        let result = field.validate_value(&value);
        assert!(result.is_err());
    }
}

mod array_field_validation {
    use super::*;

    #[test]
    fn test_array_field_accepts_array() {
        let field = create_field_definition("items", FieldType::Array);
        let value = json!([1, 2, 3]);
        assert!(field.validate_value(&value).is_ok());
    }

    #[test]
    fn test_array_field_rejects_object() {
        let field = create_field_definition("items", FieldType::Array);
        let value = json!({"key": "value"});
        let result = field.validate_value(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be an array"));
    }
}

mod boolean_field_validation {
    use super::*;

    #[test]
    fn test_boolean_field_accepts_true() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        assert!(field.validate_value(&json!(true)).is_ok());
    }

    #[test]
    fn test_boolean_field_accepts_false() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        assert!(field.validate_value(&json!(false)).is_ok());
    }

    #[test]
    fn test_boolean_field_rejects_string_true() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        let result = field.validate_value(&json!("true"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must be a boolean"));
        assert!(err.contains("string"));
    }

    #[test]
    fn test_boolean_field_rejects_string_false() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        let result = field.validate_value(&json!("false"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be a boolean"));
    }

    #[test]
    fn test_boolean_field_rejects_number() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        let result = field.validate_value(&json!(1));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be a boolean"));
    }

    #[test]
    fn test_boolean_field_rejects_string_yes() {
        let field = create_field_definition("opt_in", FieldType::Boolean);
        let result = field.validate_value(&json!("yes"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be a boolean"));
    }
}
