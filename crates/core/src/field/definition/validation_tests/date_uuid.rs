use super::*;

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
