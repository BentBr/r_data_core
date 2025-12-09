#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_services::query_validation::{
    validate_list_query, FieldValidator, ListQueryParams,
};
use r_data_core_test_support::setup_test_db;
use serial_test::serial;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test field name sanitization with valid field names
    #[test]
    fn test_sanitize_field_name_valid() {
        let valid_fields = vec![
            "username",
            "user_name",
            "user123",
            "created_at",
            "UPPER_CASE",
            "mixed_Case123",
        ];

        for field in valid_fields {
            let result = FieldValidator::sanitize_field_name(field);
            assert!(result.is_ok(), "Field '{field}' should be valid");
            assert_eq!(result.unwrap(), field);
        }
    }

    /// Test field name sanitization with invalid field names (SQL injection attempts)
    #[test]
    fn test_sanitize_field_name_invalid_sql_injection() {
        let invalid_fields = vec![
            "'; DROP TABLE users; --",
            "field; DELETE FROM users",
            "field' OR '1'='1",
            "field\" UNION SELECT * FROM users",
            "field; SELECT * FROM users",
            "field`",
            "field'",
            "field\"",
            "field;",
            "field--",
            "field/*",
            "field*/",
            "field (",
            "field )",
            "field[",
            "field]",
            "field{",
            "field}",
            "field@",
            "field#",
            "field$",
            "field%",
            "field^",
            "field&",
            "field*",
            "field+",
            "field=",
            "field|",
            "field\\",
            "field/",
            "field?",
            "field<",
            "field>",
            "field,",
            "field.",
            "field:",
            "field;",
            "field ",
            "field\t",
            "field\n",
        ];

        for field in invalid_fields {
            let result = FieldValidator::sanitize_field_name(field);
            assert!(
                result.is_err(),
                "Field '{field}' should be rejected as invalid"
            );
            let err_msg = result.unwrap_err();
            assert!(
                err_msg.contains("Invalid field name"),
                "Error message should mention invalid field name for '{field}'"
            );
        }
    }

    /// Test field name sanitization with empty field
    #[test]
    fn test_sanitize_field_name_empty() {
        let result = FieldValidator::sanitize_field_name("");
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("cannot be empty"));
    }

    /// Test field validation with existing table
    #[tokio::test]
    #[serial]
    async fn test_validate_field_existing() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // Test with admin_users table (should exist)
        let result = validator.validate_field("admin_users", "username").await;
        assert!(
            result.is_ok(),
            "username should be a valid field in admin_users"
        );
    }

    /// Test field validation with non-existing field
    #[tokio::test]
    #[serial]
    async fn test_validate_field_non_existing() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // Test with non-existing field
        let result = validator
            .validate_field("admin_users", "non_existing_field_xyz")
            .await;
        assert!(
            result.is_err(),
            "non_existing_field_xyz should not be a valid field"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Invalid sort field"),
            "Error should mention invalid sort field"
        );
        assert!(
            err_msg.contains("non_existing_field_xyz"),
            "Error should mention the invalid field name"
        );
    }

    /// Test field validation caching
    #[tokio::test]
    #[serial]
    async fn test_field_validation_caching() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // First call should query database
        let result1 = validator.get_valid_fields("admin_users").await;
        assert!(result1.is_ok());
        let fields1 = result1.unwrap();
        assert!(!fields1.is_empty(), "Should have some fields");

        // Second call should use cache (no database query)
        let result2 = validator.get_valid_fields("admin_users").await;
        assert!(result2.is_ok());
        let fields2 = result2.unwrap();

        // Should return same fields
        assert_eq!(fields1, fields2, "Cached fields should match");
    }

    /// Test field validation with different tables
    #[tokio::test]
    #[serial]
    async fn test_validate_field_different_tables() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // Test admin_users table
        let result = validator.validate_field("admin_users", "username").await;
        assert!(result.is_ok(), "username should exist in admin_users");

        // Test api_keys table if it exists
        let _result = validator.validate_field("api_keys", "name").await;
        // This might fail if table doesn't exist, but that's ok for this test
        // We're just checking that different tables are handled separately
    }

    /// Test sort order validation - valid values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_sort_order_valid() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let valid_orders = vec!["asc", "ASC", "desc", "DESC", "Asc", "DeSc"];

        for order in valid_orders {
            let params = ListQueryParams {
                page: Some(1),
                per_page: Some(20),
                limit: None,
                offset: None,
                sort_by: Some("username".to_string()),
                sort_order: Some(order.to_string()),
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_ok(), "Sort order '{order}' should be valid");
        }
    }

    /// Allow virtual sort fields when explicitly whitelisted (e.g., roles)
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_virtual_sort_field_allowed() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let params = ListQueryParams {
            page: Some(1),
            per_page: Some(20),
            limit: None,
            offset: None,
            sort_by: Some("roles".to_string()),
            sort_order: Some("asc".to_string()),
        };

        let result = validate_list_query(
            &params,
            "admin_users",
            &validator,
            20,
            100,
            true,
            &["roles"],
        )
        .await;
        assert!(
            result.is_ok(),
            "Virtual sort field 'roles' should be accepted when whitelisted"
        );
    }

    /// Test sort order validation - invalid values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_sort_order_invalid() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let invalid_orders = vec![
            "invalid",
            "ASCENDING",
            "DESCENDING",
            "up",
            "down",
            "1",
            "0",
            "",
        ];

        for order in invalid_orders {
            let params = ListQueryParams {
                page: Some(1),
                per_page: Some(20),
                limit: None,
                offset: None,
                sort_by: Some("username".to_string()),
                sort_order: Some(order.to_string()),
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_err(), "Sort order '{order}' should be invalid");
            let err_msg = result.unwrap_err();
            assert!(
                err_msg.contains("Invalid sort_order"),
                "Error should mention invalid sort_order for '{order}'"
            );
        }
    }

    /// Test pagination validation - valid page numbers
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_valid_page() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let valid_pages = vec![1, 2, 10, 100, 1000];

        for page in valid_pages {
            let params = ListQueryParams {
                page: Some(page),
                per_page: Some(20),
                limit: None,
                offset: None,
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_ok(), "Page {page} should be valid");
        }
    }

    /// Test pagination validation - invalid page numbers
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_invalid_page() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let invalid_pages = vec![0, -1, -10];

        for page in invalid_pages {
            let params = ListQueryParams {
                page: Some(page),
                per_page: Some(20),
                limit: None,
                offset: None,
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_err(), "Page {page} should be invalid");
            let err_msg = result.unwrap_err();
            assert!(
                err_msg.contains("Pagination validation failed"),
                "Error should mention pagination validation for page {page}"
            );
        }
    }

    /// Test pagination validation - valid `per_page` values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_valid_per_page() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let valid_per_pages = vec![1, 10, 20, 50, 100, -1]; // -1 for unlimited

        for per_page in valid_per_pages {
            let params = ListQueryParams {
                page: Some(1),
                per_page: Some(per_page),
                limit: None,
                offset: None,
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_ok(), "per_page {per_page} should be valid");
        }
    }

    /// Test pagination validation - invalid `per_page` values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_invalid_per_page() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let invalid_per_pages = vec![0, -2, 101, 1000];

        for per_page in invalid_per_pages {
            let params = ListQueryParams {
                page: Some(1),
                per_page: Some(per_page),
                limit: None,
                offset: None,
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_err(), "per_page {per_page} should be invalid");
        }
    }

    /// Test pagination validation - `per_page` = -1 not allowed when `allow_unlimited` = false
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_unlimited_not_allowed() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let params = ListQueryParams {
            page: Some(1),
            per_page: Some(-1),
            limit: None,
            offset: None,
            sort_by: None,
            sort_order: None,
        };

        // Should fail when allow_unlimited = false
        let result =
            validate_list_query(&params, "admin_users", &validator, 20, 100, false, &[]).await;
        assert!(
            result.is_err(),
            "per_page = -1 should be invalid when allow_unlimited = false"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("per_page = -1 is not allowed"),
            "Error should mention that per_page = -1 is not allowed"
        );
    }

    /// Test pagination validation - valid limit values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_valid_limit() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let valid_limits = vec![1, 10, 20, 50, 100];

        for limit in valid_limits {
            let params = ListQueryParams {
                page: None,
                per_page: None,
                limit: Some(limit),
                offset: Some(0),
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_ok(), "Limit {limit} should be valid");
        }
    }

    /// Test pagination validation - invalid limit values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_invalid_limit() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let invalid_limits = vec![0, -1, 101, 1000];

        for limit in invalid_limits {
            let params = ListQueryParams {
                page: None,
                per_page: None,
                limit: Some(limit),
                offset: Some(0),
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_err(), "Limit {limit} should be invalid");
        }
    }

    /// Test pagination validation - invalid offset values
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_pagination_invalid_offset() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let invalid_offsets = vec![-1, -10];

        for offset in invalid_offsets {
            let params = ListQueryParams {
                page: None,
                per_page: None,
                limit: Some(20),
                offset: Some(offset),
                sort_by: None,
                sort_order: None,
            };

            let result =
                validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
            assert!(result.is_err(), "Offset {offset} should be invalid");
        }
    }

    /// Test complete validation with all valid parameters
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_all_valid() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let params = ListQueryParams {
            page: Some(2),
            per_page: Some(25),
            limit: None,
            offset: None,
            sort_by: Some("username".to_string()),
            sort_order: Some("asc".to_string()),
        };

        let result =
            validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
        assert!(
            result.is_ok(),
            "All valid parameters should pass validation"
        );

        let validated = result.unwrap();
        assert_eq!(validated.page, 2);
        assert_eq!(validated.per_page, 25);
        assert_eq!(validated.sort_by, Some("username".to_string()));
        assert_eq!(validated.sort_order, Some("asc".to_string()));
    }

    /// Test validation with missing optional parameters (should use defaults)
    #[tokio::test]
    #[serial]
    async fn test_validate_list_query_defaults() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        let params = ListQueryParams {
            page: None,
            per_page: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_order: None,
        };

        let result =
            validate_list_query(&params, "admin_users", &validator, 20, 100, true, &[]).await;
        assert!(
            result.is_ok(),
            "Missing optional parameters should use defaults"
        );

        let validated = result.unwrap();
        assert_eq!(validated.page, 1); // Default page
        assert_eq!(validated.per_page, 20); // Default per_page
    }

    /// Test cache clearing
    #[tokio::test]
    #[serial]
    async fn test_field_validator_cache_clear() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // Populate cache
        let _ = validator.get_valid_fields("admin_users").await;

        // Clear cache for specific table
        validator.clear_cache("admin_users").await;

        // Should still work after clearing
        let result = validator.get_valid_fields("admin_users").await;
        assert!(result.is_ok());
    }

    /// Test cache clearing all
    #[tokio::test]
    #[serial]
    async fn test_field_validator_cache_clear_all() {
        let pool = setup_test_db().await;
        let validator = FieldValidator::new(Arc::new(pool.pool.clone()));

        // Populate cache for multiple tables
        let _ = validator.get_valid_fields("admin_users").await;

        // Clear all cache
        validator.clear_all_cache().await;

        // Should still work after clearing
        let result = validator.get_valid_fields("admin_users").await;
        assert!(result.is_ok());
    }
}
