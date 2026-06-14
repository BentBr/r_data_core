use super::*;

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
        let value = json!("[1, 2, 3]");
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
