#![allow(clippy::unwrap_used)]

use super::*;
use serde_json;

// ── SortingQuery ──────────────────────────────────────────────────────────────

#[test]
fn validate_sort_order_none_returns_asc() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: None,
    };
    assert_eq!(q.validate_sort_order().unwrap(), "ASC");
}

#[test]
fn validate_sort_order_asc_lowercase() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: Some("asc".to_string()),
    };
    assert_eq!(q.validate_sort_order().unwrap(), "ASC");
}

#[test]
fn validate_sort_order_desc_uppercase() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: Some("DESC".to_string()),
    };
    assert_eq!(q.validate_sort_order().unwrap(), "DESC");
}

#[test]
fn validate_sort_order_invalid_fails() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: Some("random".to_string()),
    };
    assert!(q.validate_sort_order().is_err());
}

#[test]
fn get_sort_order_falls_back_to_asc_on_invalid() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: Some("invalid_order".to_string()),
    };
    assert_eq!(q.get_sort_order(), "ASC");
}

#[test]
fn sanitize_field_name_valid() {
    assert_eq!(
        SortingQuery::sanitize_field_name("created_at").unwrap(),
        "created_at"
    );
}

#[test]
fn sanitize_field_name_empty_fails() {
    assert!(SortingQuery::sanitize_field_name("").is_err());
}

#[test]
fn sanitize_field_name_invalid_chars_fails() {
    let err = SortingQuery::sanitize_field_name("field; DROP TABLE").unwrap_err();
    assert!(!err.is_empty(), "expected non-empty error, got: {err}");
}

#[test]
fn get_sort_clause_with_sort_by() {
    let q = SortingQuery {
        sort_by: Some("name".to_string()),
        sort_order: Some("desc".to_string()),
    };
    assert_eq!(q.get_sort_clause().unwrap(), "name DESC");
}

#[test]
fn get_sort_clause_without_sort_by() {
    let q = SortingQuery {
        sort_by: None,
        sort_order: Some("asc".to_string()),
    };
    assert!(q.get_sort_clause().is_none());
}

// ── FieldsQuery ───────────────────────────────────────────────────────────────

#[test]
fn get_fields_some() {
    let q = FieldsQuery {
        fields: Some("id,name,email".to_string()),
    };
    let fields = q.get_fields().unwrap();
    assert_eq!(fields, vec!["id", "name", "email"]);
}

#[test]
fn get_fields_none() {
    let q = FieldsQuery { fields: None };
    assert!(q.get_fields().is_none());
}

#[test]
fn get_fields_empty_string_returns_empty_vec() {
    let q = FieldsQuery {
        fields: Some(String::new()),
    };
    let fields = q.get_fields().unwrap();
    assert!(fields.is_empty());
}

#[test]
fn get_fields_whitespace_only_items_filtered() {
    let q = FieldsQuery {
        fields: Some("id, ,name".to_string()),
    };
    let fields = q.get_fields().unwrap();
    assert_eq!(fields, vec!["id", "name"]);
}

// ── FilterQuery ───────────────────────────────────────────────────────────────

#[test]
fn parse_filter_none() {
    let q = FilterQuery {
        filter: None,
        q: None,
    };
    assert!(q.parse_filter().is_none());
}

#[test]
fn parse_filter_json_object() {
    let q = FilterQuery {
        filter: Some(r#"{"status":"active"}"#.to_string()),
        q: None,
    };
    let val = q.parse_filter().unwrap();
    assert_eq!(val["status"], serde_json::json!("active"));
}

#[test]
fn parse_filter_compact_syntax() {
    let q = FilterQuery {
        filter: Some("status:active,type:user".to_string()),
        q: None,
    };
    let val = q.parse_filter().unwrap();
    assert_eq!(val["status"], serde_json::json!("active"));
    assert_eq!(val["type"], serde_json::json!("user"));
}

// ── IncludeQuery ──────────────────────────────────────────────────────────────

#[test]
fn get_includes_some() {
    let q = IncludeQuery {
        include: Some("children,tags".to_string()),
        include_children_count: None,
    };
    let includes = q.get_includes().unwrap();
    assert_eq!(includes, vec!["children", "tags"]);
}

#[test]
fn get_includes_none() {
    let q = IncludeQuery {
        include: None,
        include_children_count: None,
    };
    assert!(q.get_includes().is_none());
}

#[test]
fn get_includes_empty_string_returns_empty_vec() {
    let q = IncludeQuery {
        include: Some(String::new()),
        include_children_count: None,
    };
    let includes = q.get_includes().unwrap();
    assert!(includes.is_empty());
}

// ── StandardQuery::to_limit_offset ───────────────────────────────────────────

#[test]
fn standard_query_to_limit_offset_delegates() {
    let q = StandardQuery {
        pagination: PaginationQuery {
            page: Some(2),
            per_page: Some(10),
            limit: None,
            offset: None,
        },
        sorting: SortingQuery {
            sort_by: None,
            sort_order: None,
        },
        fields: FieldsQuery { fields: None },
        filter: FilterQuery {
            filter: None,
            q: None,
        },
        include: IncludeQuery {
            include: None,
            include_children_count: None,
        },
    };
    // page=2, per_page=10 → limit=10, offset=10
    let (limit, offset) = q.to_limit_offset();
    assert_eq!(limit, 10);
    assert_eq!(offset, 10);
}

// ── deserialize_optional_bool error branch ───────────────────────────────────

#[test]
fn deserialize_optional_bool_invalid_string_errors() {
    let json = serde_json::json!({ "include_children_count": "maybe" });
    let result: Result<IncludeQuery, _> = serde_json::from_value(json);
    assert!(
        result.is_err(),
        "expected deserialization error for invalid bool string"
    );
}
