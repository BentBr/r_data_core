use super::*;

mod string_field_validation {
    use super::*;
    use crate::field::options::{FieldValidation, OptionsSource, SelectOption};

    #[test]
    fn test_rejects_non_string() {
        let f = create_field_definition("title", FieldType::String);
        assert!(f
            .validate_value(&json!(42))
            .unwrap_err()
            .to_string()
            .contains("must be a string"));
    }

    #[test]
    fn test_min_length_violation() {
        let mut f = create_field_definition("title", FieldType::String);
        f.validation = FieldValidation {
            min_length: Some(5),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("ab"))
            .unwrap_err()
            .to_string()
            .contains("at least 5 characters"));
    }

    #[test]
    fn test_max_length_violation() {
        let mut f = create_field_definition("title", FieldType::String);
        f.validation = FieldValidation {
            max_length: Some(3),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("toolong"))
            .unwrap_err()
            .to_string()
            .contains("at most 3 characters"));
    }

    #[test]
    fn test_pattern_no_match() {
        let mut f = create_field_definition("code", FieldType::String);
        f.validation = FieldValidation {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("ABC"))
            .unwrap_err()
            .to_string()
            .contains("does not match pattern"));
    }

    #[test]
    fn test_pattern_match_ok() {
        let mut f = create_field_definition("code", FieldType::String);
        f.validation = FieldValidation {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };
        assert!(f.validate_value(&json!("abc")).is_ok());
    }

    #[test]
    fn test_invalid_regex_returns_error() {
        let mut f = create_field_definition("code", FieldType::String);
        f.validation = FieldValidation {
            pattern: Some("[invalid(regex".to_string()),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("abc"))
            .unwrap_err()
            .to_string()
            .contains("Invalid pattern"));
    }

    #[test]
    fn test_empty_string_skips_pattern_when_optional() {
        let mut f = create_field_definition("code", FieldType::String);
        f.required = false;
        f.validation = FieldValidation {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };
        assert!(f.validate_value(&json!("")).is_ok());
    }

    #[test]
    fn test_options_source_rejects_invalid_value() {
        let mut f = create_field_definition("status", FieldType::String);
        f.validation = FieldValidation {
            options_source: Some(OptionsSource::Fixed {
                options: vec![SelectOption {
                    value: "active".to_string(),
                    label: "Active".to_string(),
                }],
            }),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("unknown"))
            .unwrap_err()
            .to_string()
            .contains("must be one of"));
    }

    #[test]
    fn test_options_source_accepts_valid_value() {
        let mut f = create_field_definition("status", FieldType::String);
        f.validation = FieldValidation {
            options_source: Some(OptionsSource::Fixed {
                options: vec![SelectOption {
                    value: "active".to_string(),
                    label: "Active".to_string(),
                }],
            }),
            ..Default::default()
        };
        assert!(f.validate_value(&json!("active")).is_ok());
    }

    #[test]
    fn test_text_wysiwyg_password_use_string_validation() {
        for ft in [FieldType::Text, FieldType::Wysiwyg, FieldType::Password] {
            let f = create_field_definition("f", ft.clone());
            assert!(
                f.validate_value(&json!(99)).is_err(),
                "type {ft:?} should reject non-string"
            );
        }
    }
}
