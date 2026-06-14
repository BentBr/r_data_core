use super::*;

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
