use super::*;

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
