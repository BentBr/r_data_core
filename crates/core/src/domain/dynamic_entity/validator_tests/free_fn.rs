use super::*;

mod validate_field_free_fn_tests {
    use super::*;

    #[test]
    fn test_missing_type_returns_error() {
        let r = validate_field(&json!({}), &json!("val"), "my_field");
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("Missing type for field my_field"));
    }

    #[test]
    fn test_string_type_ok() {
        assert!(validate_field(&json!({"type":"string"}), &json!("hi"), "f").is_ok());
    }

    #[test]
    fn test_string_type_rejects_number() {
        assert!(validate_field(&json!({"type":"string"}), &json!(42), "f")
            .unwrap_err()
            .to_string()
            .contains("must be a string"));
    }

    #[test]
    fn test_number_type_ok() {
        assert!(validate_field(&json!({"type":"number"}), &json!(3.5), "f").is_ok());
    }

    #[test]
    fn test_integer_type_rejects_string() {
        assert!(
            validate_field(&json!({"type":"integer"}), &json!("abc"), "f")
                .unwrap_err()
                .to_string()
                .contains("must be a number")
        );
    }

    #[test]
    fn test_boolean_type_ok() {
        assert!(validate_field(&json!({"type":"boolean"}), &json!(true), "f").is_ok());
    }

    #[test]
    fn test_boolean_type_rejects_string() {
        assert!(
            validate_field(&json!({"type":"boolean"}), &json!("yes"), "f")
                .unwrap_err()
                .to_string()
                .contains("must be a boolean")
        );
    }

    #[test]
    fn test_array_type_ok() {
        assert!(validate_field(&json!({"type":"array"}), &json!([1, 2]), "f").is_ok());
    }

    #[test]
    fn test_array_type_rejects_object() {
        assert!(
            validate_field(&json!({"type":"array"}), &json!({"k":"v"}), "f")
                .unwrap_err()
                .to_string()
                .contains("must be an array")
        );
    }

    #[test]
    fn test_object_type_ok() {
        assert!(validate_field(&json!({"type":"object"}), &json!({"k":"v"}), "f").is_ok());
    }

    #[test]
    fn test_object_type_rejects_string() {
        assert!(
            validate_field(&json!({"type":"object"}), &json!("str"), "f")
                .unwrap_err()
                .to_string()
                .contains("must be an object")
        );
    }

    #[test]
    fn test_unknown_type_accepted() {
        assert!(validate_field(&json!({"type":"custom"}), &json!("anything"), "f").is_ok());
    }
}
