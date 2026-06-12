#![allow(clippy::unwrap_used)]

use crate::field::definition::FieldDefinition;
use crate::field::options::FieldValidation;
use crate::field::ui::UiSettings;
use crate::field::FieldType;
use serde_json::json;

fn create_field_definition(name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: name.to_string(),
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

mod integer_field_validation {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_integer() {
        let f = create_field_definition("count", FieldType::Integer);
        assert!(f.validate_value(&json!(42)).is_ok());
    }

    #[test]
    fn test_accepts_string_integer() {
        let f = create_field_definition("count", FieldType::Integer);
        assert!(f.validate_value(&json!("100")).is_ok());
    }

    #[test]
    fn test_rejects_non_parseable_string() {
        let f = create_field_definition("count", FieldType::Integer);
        assert!(f
            .validate_value(&json!("abc"))
            .unwrap_err()
            .to_string()
            .contains("must be an integer"));
    }

    #[test]
    fn test_rejects_float_json_value() {
        let f = create_field_definition("count", FieldType::Integer);
        // A bare f64 in JSON (e.g. 1.5) is not i64/u64 and not a string
        assert!(f.validate_value(&json!(1.5)).is_err());
    }

    #[test]
    fn test_min_value_violation() {
        let mut f = create_field_definition("count", FieldType::Integer);
        f.validation = FieldValidation {
            min_value: Some(json!(10)),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!(5))
            .unwrap_err()
            .to_string()
            .contains("at least 10"));
    }

    #[test]
    fn test_max_value_violation() {
        let mut f = create_field_definition("count", FieldType::Integer);
        f.validation = FieldValidation {
            max_value: Some(json!(100)),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!(200))
            .unwrap_err()
            .to_string()
            .contains("at most 100"));
    }

    #[test]
    fn test_positive_only_violation() {
        let mut f = create_field_definition("count", FieldType::Integer);
        f.validation = FieldValidation {
            positive_only: Some(true),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!(-5))
            .unwrap_err()
            .to_string()
            .contains("must be positive"));
    }
}

mod float_field_validation {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_float() {
        let f = create_field_definition("price", FieldType::Float);
        assert!(f.validate_value(&json!(9.99)).is_ok());
    }

    #[test]
    fn test_accepts_string_float() {
        let f = create_field_definition("price", FieldType::Float);
        assert!(f.validate_value(&json!("2.5")).is_ok());
    }

    #[test]
    fn test_rejects_non_parseable_string() {
        let f = create_field_definition("price", FieldType::Float);
        assert!(f
            .validate_value(&json!("notanumber"))
            .unwrap_err()
            .to_string()
            .contains("must be a number"));
    }

    #[test]
    fn test_rejects_boolean() {
        let f = create_field_definition("price", FieldType::Float);
        assert!(f
            .validate_value(&json!(true))
            .unwrap_err()
            .to_string()
            .contains("must be a number"));
    }

    #[test]
    fn test_min_value_violation() {
        let mut f = create_field_definition("price", FieldType::Float);
        f.validation = FieldValidation {
            min_value: Some(json!(0.0)),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!(-1.0))
            .unwrap_err()
            .to_string()
            .contains("at least 0"));
    }

    #[test]
    fn test_positive_only_violation() {
        let mut f = create_field_definition("price", FieldType::Float);
        f.validation = FieldValidation {
            positive_only: Some(true),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!(-0.1))
            .unwrap_err()
            .to_string()
            .contains("must be positive"));
    }
}

mod date_field_validation {
    use super::*;
    use crate::field::options::FieldValidation;

    #[test]
    fn test_accepts_rfc3339_datetime() {
        let f = create_field_definition("ts", FieldType::DateTime);
        assert!(f.validate_value(&json!("2024-06-15T12:00:00Z")).is_ok());
    }

    #[test]
    fn test_rejects_non_string_date() {
        let f = create_field_definition("ts", FieldType::Date);
        assert!(f
            .validate_value(&json!(20_240_615_i64))
            .unwrap_err()
            .to_string()
            .contains("must be a date string"));
    }

    #[test]
    fn test_rejects_invalid_date_format() {
        let f = create_field_definition("ts", FieldType::Date);
        assert!(f
            .validate_value(&json!("not-a-date"))
            .unwrap_err()
            .to_string()
            .contains("RFC3339 format"));
    }

    #[test]
    fn test_min_date_violation() {
        let mut f = create_field_definition("ts", FieldType::DateTime);
        f.validation = FieldValidation {
            min_date: Some("2030-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("2020-01-01T00:00:00Z"))
            .unwrap_err()
            .to_string()
            .contains("must be after"));
    }

    #[test]
    fn test_max_date_violation() {
        let mut f = create_field_definition("ts", FieldType::DateTime);
        f.validation = FieldValidation {
            max_date: Some("2000-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(f
            .validate_value(&json!("2024-06-15T12:00:00Z"))
            .unwrap_err()
            .to_string()
            .contains("must be before"));
    }
}

mod uuid_field_validation {
    use super::*;

    #[test]
    fn test_accepts_valid_uuid() {
        let f = create_field_definition("id", FieldType::Uuid);
        assert!(f
            .validate_value(&json!("550e8400-e29b-41d4-a716-446655440000"))
            .is_ok());
    }

    #[test]
    fn test_rejects_non_string() {
        let f = create_field_definition("id", FieldType::Uuid);
        assert!(f
            .validate_value(&json!(42))
            .unwrap_err()
            .to_string()
            .contains("must be a UUID string"));
    }

    #[test]
    fn test_rejects_invalid_uuid() {
        let f = create_field_definition("id", FieldType::Uuid);
        assert!(f
            .validate_value(&json!("not-a-uuid"))
            .unwrap_err()
            .to_string()
            .contains("must be a valid UUID"));
    }
}

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
