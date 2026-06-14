use super::*;

mod field_definition_validate_tests {
    use super::*;

    #[test]
    fn test_empty_name_rejected() {
        let mut f = create_field_definition("", FieldType::String);
        f.display_name = "Valid".to_string();
        assert!(f
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Field name cannot be empty"));
    }

    #[test]
    fn test_empty_display_name_rejected() {
        let mut f = create_field_definition("valid_name", FieldType::String);
        f.display_name = String::new();
        assert!(f
            .validate()
            .unwrap_err()
            .to_string()
            .contains("display name cannot be empty"));
    }

    #[test]
    fn test_reserved_keyword_rejected() {
        let f = create_field_definition("select", FieldType::String);
        assert!(f
            .validate()
            .unwrap_err()
            .to_string()
            .contains("reserved SQL keyword"));
    }

    #[test]
    fn test_valid_field_definition_passes() {
        let f = create_field_definition("product_name", FieldType::String);
        assert!(f.validate().is_ok());
    }

    #[test]
    fn test_reserved_keyword_case_insensitive() {
        let f = create_field_definition("SELECT", FieldType::String);
        assert!(f
            .validate()
            .unwrap_err()
            .to_string()
            .contains("reserved SQL keyword"));
    }
}
