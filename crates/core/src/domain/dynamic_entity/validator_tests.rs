#![allow(clippy::unwrap_used)]

use super::validator::*;
use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use serde_json::json;

fn create_test_field(field_type: FieldType, required: bool) -> FieldDefinition {
    FieldDefinition {
        name: "test_field".to_string(),
        display_name: "Test Field".to_string(),
        field_type,
        required,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}

fn create_named_field(name: &str, display_name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        field_type,
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: None,
        validation: crate::field::FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: std::collections::HashMap::new(),
        description: None,
    }
}

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

// ── new coverage ──────────────────────────────────────────────────────────────

mod validate_string_tests {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_rejects_non_string() {
        let f = create_test_field(FieldType::String, false);
        let r = DynamicEntityValidator::validate_field(&f, &json!(42));
        assert!(r.unwrap_err().to_string().contains("must be a string"));
    }

    #[test]
    fn test_required_null_rejected() {
        let f = create_test_field(FieldType::String, true);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(null))
            .unwrap_err()
            .to_string()
            .contains("is required"));
    }

    #[test]
    fn test_required_empty_string_rejected() {
        let f = create_test_field(FieldType::String, true);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(""))
            .unwrap_err()
            .to_string()
            .contains("is required"));
    }

    #[test]
    fn test_min_length_violation() {
        let mut f = create_test_field(FieldType::String, false);
        f.validation = FieldValidation {
            min_length: Some(5),
            ..Default::default()
        };
        let r = DynamicEntityValidator::validate_field(&f, &json!("ab"));
        assert!(r.unwrap_err().to_string().contains("at least 5 characters"));
    }

    #[test]
    fn test_max_length_violation() {
        let mut f = create_test_field(FieldType::String, false);
        f.validation = FieldValidation {
            max_length: Some(3),
            ..Default::default()
        };
        let r = DynamicEntityValidator::validate_field(&f, &json!("toolong"));
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("no more than 3 characters"));
    }

    #[test]
    fn test_pattern_no_match_error() {
        let mut f = create_test_field(FieldType::String, false);
        f.validation = FieldValidation {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };
        let r = DynamicEntityValidator::validate_field(&f, &json!("ABC123"));
        assert!(r.unwrap_err().to_string().contains("must match pattern"));
    }

    #[test]
    fn test_pattern_match_ok() {
        let mut f = create_test_field(FieldType::String, false);
        f.validation = FieldValidation {
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!("abc")).is_ok());
    }

    #[test]
    fn test_invalid_regex_error() {
        let mut f = create_test_field(FieldType::String, false);
        f.validation = FieldValidation {
            pattern: Some("[invalid(regex".to_string()),
            ..Default::default()
        };
        let r = DynamicEntityValidator::validate_field(&f, &json!("abc"));
        assert!(r.unwrap_err().to_string().contains("invalid regex pattern"));
    }
}

mod validate_integer_tests {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_integer() {
        let f = create_test_field(FieldType::Integer, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(42)).is_ok());
    }

    #[test]
    fn test_accepts_string_integer() {
        let f = create_test_field(FieldType::Integer, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("123")).is_ok());
    }

    #[test]
    fn test_rejects_non_parseable_string() {
        let f = create_test_field(FieldType::Integer, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("abc"))
            .unwrap_err()
            .to_string()
            .contains("valid integer"));
    }

    #[test]
    fn test_rejects_boolean() {
        let f = create_test_field(FieldType::Integer, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(true))
            .unwrap_err()
            .to_string()
            .contains("must be an integer"));
    }

    #[test]
    fn test_min_value_violation() {
        let mut f = create_test_field(FieldType::Integer, false);
        f.validation = FieldValidation {
            min_value: Some(json!(10)),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!(5))
            .unwrap_err()
            .to_string()
            .contains("at least 10"));
    }

    #[test]
    fn test_max_value_violation() {
        let mut f = create_test_field(FieldType::Integer, false);
        f.validation = FieldValidation {
            max_value: Some(json!(100)),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!(200))
            .unwrap_err()
            .to_string()
            .contains("no more than 100"));
    }

    #[test]
    fn test_positive_only_violation() {
        let mut f = create_test_field(FieldType::Integer, false);
        f.validation = FieldValidation {
            positive_only: Some(true),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!(-1))
            .unwrap_err()
            .to_string()
            .contains("positive number"));
    }
}

mod validate_float_tests {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_float() {
        let f = create_test_field(FieldType::Float, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(3.5)).is_ok());
    }

    #[test]
    fn test_accepts_string_float() {
        let f = create_test_field(FieldType::Float, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("2.718")).is_ok());
    }

    #[test]
    fn test_rejects_non_parseable_string() {
        let f = create_test_field(FieldType::Float, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("nan-val"))
                .unwrap_err()
                .to_string()
                .contains("valid number")
        );
    }

    #[test]
    fn test_rejects_boolean() {
        let f = create_test_field(FieldType::Float, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(true))
            .unwrap_err()
            .to_string()
            .contains("must be a number"));
    }

    #[test]
    fn test_min_max_range_ok() {
        let mut f = create_test_field(FieldType::Float, false);
        f.validation = FieldValidation {
            min_value: Some(json!(0.0)),
            max_value: Some(json!(1.0)),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!(0.5)).is_ok());
    }
}

mod validate_boolean_tests {
    use super::*;

    #[test]
    fn test_accepts_true_false() {
        let f = create_test_field(FieldType::Boolean, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(true)).is_ok());
        assert!(DynamicEntityValidator::validate_field(&f, &json!(false)).is_ok());
    }

    #[test]
    fn test_accepts_truthy_strings() {
        let f = create_test_field(FieldType::Boolean, false);
        for s in &["true", "yes", "1", "false", "no", "0"] {
            assert!(
                DynamicEntityValidator::validate_field(&f, &json!(s)).is_ok(),
                "should accept '{s}'"
            );
        }
    }

    #[test]
    fn test_rejects_invalid_string() {
        let f = create_test_field(FieldType::Boolean, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("maybe"))
            .unwrap_err()
            .to_string()
            .contains("must be a boolean value"));
    }

    #[test]
    fn test_accepts_number_0_and_1() {
        let f = create_test_field(FieldType::Boolean, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(0)).is_ok());
        assert!(DynamicEntityValidator::validate_field(&f, &json!(1)).is_ok());
    }

    #[test]
    fn test_rejects_number_2() {
        let f = create_test_field(FieldType::Boolean, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(2))
            .unwrap_err()
            .to_string()
            .contains("must be a boolean value"));
    }

    #[test]
    fn test_rejects_array() {
        let f = create_test_field(FieldType::Boolean, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!([true]))
            .unwrap_err()
            .to_string()
            .contains("must be a boolean"));
    }
}

mod validate_date_tests {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_valid_date() {
        let f = create_test_field(FieldType::Date, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("2024-06-15")).is_ok());
    }

    #[test]
    fn test_rejects_non_string() {
        let f = create_test_field(FieldType::Date, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!(20_240_615))
                .unwrap_err()
                .to_string()
                .contains("must be a date string")
        );
    }

    #[test]
    fn test_rejects_invalid_format() {
        let f = create_test_field(FieldType::Date, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("not-a-date"))
                .unwrap_err()
                .to_string()
                .contains("YYYY-MM-DD format")
        );
    }

    #[test]
    fn test_min_date_literal_violation() {
        let mut f = create_test_field(FieldType::Date, false);
        f.validation = FieldValidation {
            min_date: Some("2030-01-01".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2020-01-01"))
                .unwrap_err()
                .to_string()
                .contains("on or after")
        );
    }

    #[test]
    fn test_max_date_literal_violation() {
        let mut f = create_test_field(FieldType::Date, false);
        f.validation = FieldValidation {
            max_date: Some("2000-01-01".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2024-06-15"))
                .unwrap_err()
                .to_string()
                .contains("on or before")
        );
    }

    #[test]
    fn test_min_date_now_keyword_rejects_past() {
        let mut f = create_test_field(FieldType::Date, false);
        f.validation = FieldValidation {
            min_date: Some("now".to_string()),
            ..Default::default()
        };
        assert!(DynamicEntityValidator::validate_field(&f, &json!("2000-01-01")).is_err());
    }

    #[test]
    fn test_invalid_min_date_format() {
        let mut f = create_test_field(FieldType::Date, false);
        f.validation = FieldValidation {
            min_date: Some("bad-date".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2024-06-15"))
                .unwrap_err()
                .to_string()
                .contains("Invalid min_date format")
        );
    }

    #[test]
    fn test_invalid_max_date_format() {
        let mut f = create_test_field(FieldType::Date, false);
        f.validation = FieldValidation {
            max_date: Some("bad-date".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2024-06-15"))
                .unwrap_err()
                .to_string()
                .contains("Invalid max_date format")
        );
    }
}

mod validate_datetime_tests {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_valid_rfc3339() {
        let f = create_test_field(FieldType::DateTime, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!("2024-06-15T12:00:00Z")).is_ok());
    }

    #[test]
    fn test_rejects_non_string() {
        let f = create_test_field(FieldType::DateTime, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!(1_718_445_600))
                .unwrap_err()
                .to_string()
                .contains("must be a datetime string")
        );
    }

    #[test]
    fn test_rejects_invalid_format() {
        let f = create_test_field(FieldType::DateTime, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2024-06-15"))
                .unwrap_err()
                .to_string()
                .contains("RFC3339 format")
        );
    }

    #[test]
    fn test_min_datetime_violation() {
        let mut f = create_test_field(FieldType::DateTime, false);
        f.validation = FieldValidation {
            min_date: Some("2030-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2020-01-01T00:00:00Z"))
                .unwrap_err()
                .to_string()
                .contains("on or after")
        );
    }

    #[test]
    fn test_max_datetime_violation() {
        let mut f = create_test_field(FieldType::DateTime, false);
        f.validation = FieldValidation {
            max_date: Some("2000-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2024-06-15T12:00:00Z"))
                .unwrap_err()
                .to_string()
                .contains("on or before")
        );
    }

    #[test]
    fn test_min_datetime_now_keyword_rejects_past() {
        let mut f = create_test_field(FieldType::DateTime, false);
        f.validation = FieldValidation {
            min_date: Some("now".to_string()),
            ..Default::default()
        };
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("2000-01-01T00:00:00Z")).is_err()
        );
    }
}

mod validate_uuid_tests {
    use super::*;

    #[test]
    fn test_accepts_valid_uuid() {
        let f = create_test_field(FieldType::Uuid, false);
        assert!(DynamicEntityValidator::validate_field(
            &f,
            &json!("550e8400-e29b-41d4-a716-446655440000")
        )
        .is_ok());
    }

    #[test]
    fn test_rejects_non_string() {
        let f = create_test_field(FieldType::Uuid, false);
        assert!(DynamicEntityValidator::validate_field(&f, &json!(42))
            .unwrap_err()
            .to_string()
            .contains("must be a UUID string"));
    }

    #[test]
    fn test_rejects_invalid_uuid() {
        let f = create_test_field(FieldType::Uuid, false);
        assert!(
            DynamicEntityValidator::validate_field(&f, &json!("not-a-uuid"))
                .unwrap_err()
                .to_string()
                .contains("must be a valid UUID")
        );
    }
}

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

mod validate_entity_tests {
    use super::*;
    use crate::entity_definition::definition::EntityDefinition;

    fn entity_def_with_required_string() -> EntityDefinition {
        EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![create_test_field(FieldType::String, true)],
            published: true,
            ..EntityDefinition::default()
        }
    }

    #[test]
    fn test_missing_entity_type_field() {
        let ed = entity_def_with_required_string();
        assert!(validate_entity(&json!({"field_data":{}}), &ed)
            .unwrap_err()
            .to_string()
            .contains("entity_type field"));
    }

    #[test]
    fn test_entity_type_mismatch() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"wrong","field_data":{}});
        assert!(validate_entity(&e, &ed)
            .unwrap_err()
            .to_string()
            .contains("does not match entity definition type"));
    }

    #[test]
    fn test_missing_field_data() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product"});
        assert!(validate_entity(&e, &ed)
            .unwrap_err()
            .to_string()
            .contains("field_data object"));
    }

    #[test]
    fn test_required_field_missing_creates_violation() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product","field_data":{}});
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "test_field");
        assert!(violations[0].message.contains("required"));
    }

    #[test]
    fn test_unknown_field_creates_violation() {
        let ed = EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![],
            published: true,
            ..EntityDefinition::default()
        };
        let e = json!({"entity_type":"product","field_data":{"unknown_field":"val"}});
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("not defined"));
    }

    #[test]
    fn test_system_fields_skipped() {
        let ed = EntityDefinition {
            entity_type: "product".to_string(),
            display_name: "Product".to_string(),
            fields: vec![],
            published: true,
            ..EntityDefinition::default()
        };
        let e = json!({
            "entity_type": "product",
            "field_data": {
                "uuid": "550e8400-e29b-41d4-a716-446655440000",
                "entity_key": "k",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "published": true,
                "version": 1,
                "parent_uuid": null
            }
        });
        let violations = validate_entity_with_violations(&e, &ed).unwrap();
        assert!(violations.is_empty(), "system fields should be skipped");
    }

    #[test]
    fn test_validate_entity_formats_violation_message() {
        let ed = entity_def_with_required_string();
        let e = json!({"entity_type":"product","field_data":{}});
        let msg = validate_entity(&e, &ed).unwrap_err().to_string();
        assert!(msg.contains("Validation failed with the following errors"));
    }
}

mod validate_parent_path_tests {
    use super::*;

    #[test]
    fn test_no_parent_no_violations() {
        assert!(validate_parent_path_consistency(None, None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_empty_parent_uuid_no_violations() {
        assert!(
            validate_parent_path_consistency(Some(String::new()), None, None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_parent_without_expected_path_no_violations() {
        assert!(validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&"some/path".to_string()),
            None
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn test_path_matches_expected_no_violations() {
        let path = "parent/child".to_string();
        assert!(validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&path),
            Some(&path)
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn test_path_mismatch_creates_violation() {
        let expected = "parent/child".to_string();
        let actual = "wrong/path".to_string();
        let violations = validate_parent_path_consistency(
            Some("uuid".to_string()),
            Some(&actual),
            Some(&expected),
        )
        .unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "path");
        assert!(violations[0].message.contains("Expected"));
    }

    #[test]
    fn test_missing_path_with_expected_creates_violation() {
        let expected = "parent/child".to_string();
        let violations =
            validate_parent_path_consistency(Some("uuid".to_string()), None, Some(&expected))
                .unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("required when parent_uuid"));
    }
}

mod validate_entity_with_violations_tests {
    use super::*;
    use crate::entity_definition::definition::EntityDefinition;

    fn create_test_entity_definition() -> EntityDefinition {
        EntityDefinition {
            entity_type: "test_entity".to_string(),
            display_name: "Test Entity".to_string(),
            fields: vec![
                create_test_field(FieldType::String, false),
                create_named_field("json_content", "JSON Content", FieldType::Json),
                create_named_field("items", "Items", FieldType::Array),
            ],
            published: true,
            ..EntityDefinition::default()
        }
    }

    #[test]
    fn test_json_field_accepts_string_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": "a plain string value"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(
            violations.is_empty(),
            "Json field should accept string values"
        );
    }

    #[test]
    fn test_json_field_accepts_array_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": [{"key": "value"}, {"key": "value2"}]
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(
            violations.is_empty(),
            "Json field should accept array values"
        );
    }

    #[test]
    fn test_violations_for_array_string_value() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": "[1, 2, 3]"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].field, "items");
        assert!(violations[0].message.contains("must be an array"));
    }

    #[test]
    fn test_no_violations_for_valid_json_object() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": {"key": "value", "count": 5}
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(violations.is_empty());
    }

    #[test]
    fn test_no_violations_for_valid_array() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": [{"id": 1}, {"id": 2}]
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert!(violations.is_empty());
    }

    #[test]
    fn test_violations_for_invalid_array_field() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "json_content": "valid json string value",
                "items": "not an array"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);
        assert!(violations
            .iter()
            .map(|v| v.field.as_str())
            .any(|x| x == "items"));
    }

    #[test]
    fn test_violation_message_format_for_array() {
        let entity_def = create_test_entity_definition();
        let entity = json!({
            "entity_type": "test_entity",
            "field_data": {
                "items": "not an array"
            }
        });

        let result = validate_entity_with_violations(&entity, &entity_def);
        assert!(result.is_ok());
        let violations = result.unwrap();
        assert_eq!(violations.len(), 1);

        let msg = &violations[0].message;
        assert_eq!(msg, "must be an array");
    }
}
