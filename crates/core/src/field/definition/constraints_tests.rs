#![allow(clippy::unwrap_used)]

use crate::field::definition::FieldDefinition;
use crate::field::options::FieldValidation;
use crate::field::types::FieldType;
use crate::field::ui::UiSettings;
use serde_json::json;

fn create_field_definition(field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: "test_field".to_string(),
        display_name: "Test Field".to_string(),
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

mod date_constraints {
    use super::*;

    #[test]
    fn test_datetime_min_date_accepts_string() {
        let field = create_field_definition(FieldType::DateTime);
        assert!(field
            .handle_constraint("min_date", &json!("2024-01-01"))
            .is_ok());
    }

    #[test]
    fn test_datetime_max_date_accepts_string() {
        let field = create_field_definition(FieldType::DateTime);
        assert!(field
            .handle_constraint("max_date", &json!("2024-12-31"))
            .is_ok());
    }

    #[test]
    fn test_date_min_date_accepts_string() {
        let field = create_field_definition(FieldType::Date);
        assert!(field
            .handle_constraint("min_date", &json!("2024-01-01"))
            .is_ok());
    }

    #[test]
    fn test_date_max_date_accepts_string() {
        let field = create_field_definition(FieldType::Date);
        assert!(field
            .handle_constraint("max_date", &json!("2024-12-31"))
            .is_ok());
    }

    #[test]
    fn test_datetime_min_date_rejects_number() {
        let field = create_field_definition(FieldType::DateTime);
        let result = field.handle_constraint("min_date", &json!(12345));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("String constraint"));
    }

    #[test]
    fn test_date_max_date_rejects_boolean() {
        let field = create_field_definition(FieldType::Date);
        let result = field.handle_constraint("max_date", &json!(true));
        assert!(result.is_err());
    }

    #[test]
    fn test_datetime_unknown_constraint_is_ignored() {
        let field = create_field_definition(FieldType::DateTime);
        assert!(field
            .handle_constraint("unknown", &json!("anything"))
            .is_ok());
    }
}

mod select_constraints {
    use super::*;

    #[test]
    fn test_select_options_accepts_array() {
        let field = create_field_definition(FieldType::Select);
        assert!(field
            .handle_constraint("options", &json!(["a", "b", "c"]))
            .is_ok());
    }

    #[test]
    fn test_multiselect_options_accepts_array() {
        let field = create_field_definition(FieldType::MultiSelect);
        assert!(field
            .handle_constraint("options", &json!(["x", "y"]))
            .is_ok());
    }

    #[test]
    fn test_select_options_rejects_string() {
        let field = create_field_definition(FieldType::Select);
        let result = field.handle_constraint("options", &json!("not an array"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Array constraint"));
    }

    #[test]
    fn test_multiselect_options_rejects_object() {
        let field = create_field_definition(FieldType::MultiSelect);
        let result = field.handle_constraint("options", &json!({"key": "val"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_select_unknown_constraint_is_ignored() {
        let field = create_field_definition(FieldType::Select);
        assert!(field.handle_constraint("unknown", &json!(42)).is_ok());
    }
}

mod relation_constraints {
    use super::*;

    #[test]
    fn test_many_to_one_target_class_accepts_string() {
        let field = create_field_definition(FieldType::ManyToOne);
        assert!(field
            .handle_constraint("target_class", &json!("Customer"))
            .is_ok());
    }

    #[test]
    fn test_many_to_many_target_class_accepts_string() {
        let field = create_field_definition(FieldType::ManyToMany);
        assert!(field
            .handle_constraint("target_class", &json!("Tag"))
            .is_ok());
    }

    #[test]
    fn test_many_to_one_target_class_rejects_number() {
        let field = create_field_definition(FieldType::ManyToOne);
        let result = field.handle_constraint("target_class", &json!(123));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("String constraint"));
    }

    #[test]
    fn test_many_to_many_unknown_constraint_is_ignored() {
        let field = create_field_definition(FieldType::ManyToMany);
        assert!(field
            .handle_constraint("unknown", &json!("anything"))
            .is_ok());
    }
}

mod schema_constraints {
    use super::*;

    #[test]
    fn test_object_schema_accepts_object() {
        let field = create_field_definition(FieldType::Object);
        assert!(field
            .handle_constraint("schema", &json!({"type": "object"}))
            .is_ok());
    }

    #[test]
    fn test_array_schema_accepts_object() {
        let field = create_field_definition(FieldType::Array);
        assert!(field
            .handle_constraint("schema", &json!({"items": {"type": "string"}}))
            .is_ok());
    }

    #[test]
    fn test_json_schema_accepts_object() {
        let field = create_field_definition(FieldType::Json);
        assert!(field
            .handle_constraint("schema", &json!({"type": "any"}))
            .is_ok());
    }

    #[test]
    fn test_object_schema_rejects_string() {
        let field = create_field_definition(FieldType::Object);
        let result = field.handle_constraint("schema", &json!("not an object"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Object constraint"));
    }

    #[test]
    fn test_array_schema_rejects_array() {
        let field = create_field_definition(FieldType::Array);
        let result = field.handle_constraint("schema", &json!([1, 2, 3]));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_unknown_constraint_is_ignored() {
        let field = create_field_definition(FieldType::Json);
        assert!(field
            .handle_constraint("unknown", &json!("anything"))
            .is_ok());
    }
}

mod cross_type_guard_coverage {
    use super::*;

    /// Verify that date constraints are not applied to non-date types
    #[test]
    fn test_string_field_ignores_min_date() {
        let field = create_field_definition(FieldType::String);
        assert!(field.handle_constraint("min_date", &json!(123)).is_ok());
    }

    /// Verify that options constraint is not applied to non-select types
    #[test]
    fn test_integer_field_ignores_options() {
        let field = create_field_definition(FieldType::Integer);
        assert!(field.handle_constraint("options", &json!("bad")).is_ok());
    }

    /// Verify that `target_class` constraint is not applied to non-relation types
    #[test]
    fn test_boolean_field_ignores_target_class() {
        let field = create_field_definition(FieldType::Boolean);
        assert!(field.handle_constraint("target_class", &json!(42)).is_ok());
    }

    /// Verify that schema constraint is not applied to non-structured types
    #[test]
    fn test_float_field_ignores_schema() {
        let field = create_field_definition(FieldType::Float);
        assert!(field.handle_constraint("schema", &json!("bad")).is_ok());
    }
}
