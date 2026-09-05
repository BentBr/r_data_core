use super::*;

mod select_field_validation {
    use super::*;
    use crate::field::options::{FieldValidation, OptionsSource, SelectOption};

    fn select_field() -> FieldDefinition {
        let mut f = create_field_definition("status", FieldType::Select);
        f.validation = FieldValidation {
            options_source: Some(OptionsSource::Fixed {
                options: vec![
                    SelectOption {
                        value: "open".to_string(),
                        label: "Open".to_string(),
                    },
                    SelectOption {
                        value: "closed".to_string(),
                        label: "Closed".to_string(),
                    },
                ],
            }),
            ..Default::default()
        };
        f
    }

    #[test]
    fn test_rejects_non_string() {
        let f = create_field_definition("status", FieldType::Select);
        assert!(f
            .validate_value(&json!(1))
            .unwrap_err()
            .to_string()
            .contains("must be a string"));
    }

    #[test]
    fn test_accepts_valid_option() {
        assert!(select_field().validate_value(&json!("open")).is_ok());
    }

    #[test]
    fn test_rejects_invalid_option() {
        assert!(select_field()
            .validate_value(&json!("unknown"))
            .unwrap_err()
            .to_string()
            .contains("must be one of"));
    }
}

mod multiselect_field_validation {
    use super::*;
    use crate::field::options::{FieldValidation, OptionsSource, SelectOption};

    fn ms_field() -> FieldDefinition {
        let mut f = create_field_definition("tags", FieldType::MultiSelect);
        f.validation = FieldValidation {
            options_source: Some(OptionsSource::Fixed {
                options: vec![
                    SelectOption {
                        value: "a".to_string(),
                        label: "A".to_string(),
                    },
                    SelectOption {
                        value: "b".to_string(),
                        label: "B".to_string(),
                    },
                ],
            }),
            ..Default::default()
        };
        f
    }

    #[test]
    fn test_rejects_non_array() {
        let f = create_field_definition("tags", FieldType::MultiSelect);
        assert!(f
            .validate_value(&json!("a"))
            .unwrap_err()
            .to_string()
            .contains("must be an array"));
    }

    #[test]
    fn test_rejects_array_with_non_string() {
        let f = create_field_definition("tags", FieldType::MultiSelect);
        assert!(f
            .validate_value(&json!([1, 2]))
            .unwrap_err()
            .to_string()
            .contains("must contain only string values"));
    }

    #[test]
    fn test_accepts_valid_options_array() {
        assert!(ms_field().validate_value(&json!(["a", "b"])).is_ok());
    }

    #[test]
    fn test_rejects_invalid_option_in_array() {
        assert!(ms_field()
            .validate_value(&json!(["a", "z"]))
            .unwrap_err()
            .to_string()
            .contains("must be one of"));
    }
}
