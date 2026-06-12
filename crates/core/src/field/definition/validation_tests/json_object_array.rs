use super::*;

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
