#![allow(clippy::unwrap_used)]

use super::*;
use serde_json;

#[test]
fn test_pagination_query_methods() {
    let query = PaginationQuery {
        page: Some(2),
        per_page: Some(50),
        limit: None,
        offset: None,
    };

    // Test get_page with default
    assert_eq!(query.get_page(1), 2);
    assert_eq!(query.get_page(10), 2);

    // Test get_per_page with default and max limit
    assert_eq!(query.get_per_page(20, 100), 50);
    assert_eq!(query.get_per_page(10, 30), 30); // Should be clamped to max

    // Test to_limit_offset
    let (limit, offset) = query.to_limit_offset(1, 100);
    assert_eq!(limit, 50);
    assert_eq!(offset, 50); // (page - 1) * per_page = (2 - 1) * 50 = 50
}

#[test]
fn test_pagination_query_with_none_values() {
    let query = PaginationQuery {
        page: None,
        per_page: None,
        limit: None,
        offset: None,
    };

    // Test get_page with default
    assert_eq!(query.get_page(1), 1);
    assert_eq!(query.get_page(10), 10);

    // Test get_per_page with default and max limit
    assert_eq!(query.get_per_page(20, 100), 20);
    assert_eq!(query.get_per_page(10, 30), 10);

    // Test to_limit_offset
    let (limit, offset) = query.to_limit_offset(20, 100);
    assert_eq!(limit, 20);
    assert_eq!(offset, 0); // (page - 1) * per_page = (1 - 1) * 20 = 0
}

#[test]
fn test_pagination_query_edge_cases() {
    // Test page 0 (should be clamped to 1)
    let query = PaginationQuery {
        page: Some(0),
        per_page: Some(10),
        limit: None,
        offset: None,
    };
    assert_eq!(query.get_page(1), 1); // Should be clamped to minimum 1

    // Test per_page 0 (should be clamped to 1)
    let query = PaginationQuery {
        page: Some(1),
        per_page: Some(0),
        limit: None,
        offset: None,
    };
    assert_eq!(query.get_per_page(20, 100), 1); // Should be clamped to minimum 1

    // Test very large numbers
    let query = PaginationQuery {
        page: Some(999_999),
        per_page: Some(999_999),
        limit: None,
        offset: None,
    };
    assert_eq!(query.get_page(1), 999_999);
    assert_eq!(query.get_per_page(20, 100), 100); // Should be clamped to max 100
}

#[test]
fn test_pagination_query_manual_construction() {
    // Test manually constructed PaginationQuery with string-converted values
    let query = PaginationQuery {
        page: Some(1),
        per_page: Some(1000),
        limit: None,
        offset: None,
    };

    assert_eq!(query.page, Some(1));
    assert_eq!(query.per_page, Some(1000));

    // Test the methods work correctly
    assert_eq!(query.get_page(1), 1);
    assert_eq!(query.get_per_page(20, 100), 100); // Should be clamped to max 100
}

#[test]
fn test_deserializer_integration() {
    // Test that the deserializer works with the actual struct
    // This simulates what happens when query parameters are deserialized

    // Test with string values (simulating query parameters)
    let json = serde_json::json!({
        "page": "1",
        "per_page": "1000",
        "limit": null,
        "offset": null
    });

    // This should work because our deserializer handles string-to-i64 conversion
    let result: PaginationQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.page, Some(1));
    assert_eq!(result.per_page, Some(1000));
}

#[test]
fn test_limit_offset_parameters() {
    // Test with limit/offset parameters
    let json = serde_json::json!({
        "limit": "50",
        "offset": "100",
        "page": null,
        "per_page": null
    });

    let result: PaginationQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.limit, Some(50));
    assert_eq!(result.offset, Some(100));
    assert_eq!(result.page, None);
    assert_eq!(result.per_page, None);
}

#[test]
fn test_mixed_parameters() {
    // Test with mixed parameters (should prioritize page/per_page)
    let json = serde_json::json!({
        "page": "2",
        "per_page": "25",
        "limit": "50",
        "offset": "100"
    });

    let result: PaginationQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.page, Some(2));
    assert_eq!(result.per_page, Some(25));
    assert_eq!(result.limit, Some(50));
    assert_eq!(result.offset, Some(100));
}

#[test]
fn test_include_query_with_boolean_string() {
    // Test that the deserializer handles string "true"/"false" for boolean fields
    let json = serde_json::json!({
        "include": "children",
        "include_children_count": "true"
    });

    let result: IncludeQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.include, Some("children".to_string()));
    assert_eq!(result.include_children_count, Some(true));
    assert!(result.should_include_children_count());
}

#[test]
fn test_include_query_with_boolean_false_string() {
    let json = serde_json::json!({
        "include_children_count": "false"
    });

    let result: IncludeQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.include_children_count, Some(false));
    assert!(!result.should_include_children_count());
}

#[test]
fn test_include_query_with_missing_boolean() {
    let json = serde_json::json!({
        "include": "children"
    });

    let result: IncludeQuery = serde_json::from_value(json).unwrap();
    assert_eq!(result.include_children_count, None);
    assert!(!result.should_include_children_count());
}
