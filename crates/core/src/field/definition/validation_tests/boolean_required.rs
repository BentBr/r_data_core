use super::*;

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

mod required_and_null_validation {
    use super::*;

    #[test]
    fn test_required_null_rejected() {
        let mut f = create_field_definition("name", FieldType::String);
        f.required = true;
        assert!(f
            .validate_value(&json!(null))
            .unwrap_err()
            .to_string()
            .contains("is required"));
    }

    #[test]
    fn test_optional_null_accepted() {
        let f = create_field_definition("name", FieldType::String);
        assert!(f.validate_value(&json!(null)).is_ok());
    }

    #[test]
    fn test_image_file_manytomany_skip_validation() {
        for ft in [FieldType::Image, FieldType::File, FieldType::ManyToMany] {
            let f = create_field_definition("f", ft);
            assert!(f.validate_value(&json!("anything")).is_ok());
        }
    }
}
