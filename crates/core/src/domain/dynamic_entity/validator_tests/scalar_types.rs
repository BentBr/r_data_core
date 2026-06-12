use super::*;

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
