use super::*;

mod validate_select_tests {
    use super::*;
    use crate::field::options::{FieldValidation, OptionsSource, SelectOption};

    fn select_field_with_options() -> FieldDefinition {
        let mut f = create_test_field(FieldType::Select, false);
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
    fn test_rejects_non_string() {
        let f = create_test_field(FieldType::Select, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(42))
            .unwrap_err()
            .to_string()
            .contains("must be a string"));
    }

    #[test]
    fn test_accepts_valid_option() {
        assert!(
            DynamicEntityValidator::validate_field(&select_field_with_options(), &json!("a"))
                .is_ok()
        );
    }

    #[test]
    fn test_rejects_invalid_option() {
        assert!(
            DynamicEntityValidator::validate_field(&select_field_with_options(), &json!("c"))
                .unwrap_err()
                .to_string()
                .contains("must be one of")
        );
    }

    #[test]
    fn test_accepts_any_string_without_options() {
        let f = create_test_field(FieldType::Select, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("anything")).is_ok());
    }
}

mod validate_multi_select_tests {
    use super::*;
    use crate::field::options::{FieldValidation, OptionsSource, SelectOption};

    fn ms_field_with_options() -> FieldDefinition {
        let mut f = create_test_field(FieldType::MultiSelect, false);
        f.validation = FieldValidation {
            options_source: Some(OptionsSource::Fixed {
                options: vec![
                    SelectOption {
                        value: "x".to_string(),
                        label: "X".to_string(),
                    },
                    SelectOption {
                        value: "y".to_string(),
                        label: "Y".to_string(),
                    },
                ],
            }),
            ..Default::default()
        };
        f
    }

    #[test]
    fn test_accepts_valid_array() {
        assert!(DynamicEntityValidator::validate_field(
            &ms_field_with_options(),
            &json!(["x", "y"])
        )
        .is_ok());
    }

    #[test]
    fn test_accepts_single_string() {
        assert!(
            DynamicEntityValidator::validate_field(&ms_field_with_options(), &json!("x")).is_ok()
        );
    }

    #[test]
    fn test_rejects_non_string_non_array() {
        let f = create_test_field(FieldType::MultiSelect, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(42))
            .unwrap_err()
            .to_string()
            .contains("must be an array of strings"));
    }

    #[test]
    fn test_rejects_array_with_non_string_element() {
        let f = create_test_field(FieldType::MultiSelect, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!([1, 2]))
            .unwrap_err()
            .to_string()
            .contains("must contain only strings"));
    }

    #[test]
    fn test_rejects_invalid_option() {
        assert!(DynamicEntityValidator::validate_field(
            &ms_field_with_options(),
            &json!(["x", "z"])
        )
        .unwrap_err()
        .to_string()
        .contains("contains invalid option"));
    }
}
