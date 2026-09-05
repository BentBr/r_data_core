//! Unit tests for `ApiResponse` struct-level constructors and `PaginationMeta`.
#![allow(clippy::unwrap_used)]

use crate::response::{ApiResponse, PaginationMeta, ResponseMeta, Status, ValidationViolation};

// ── Status enum ──────────────────────────────────────────────────────────────

#[test]
fn status_variants_are_distinct() {
    assert_eq!(Status::Success, Status::Success);
    assert_eq!(Status::Error, Status::Error);
    assert_ne!(Status::Success, Status::Error);
}

#[test]
fn status_serialises_to_string() {
    let s = serde_json::to_string(&Status::Success).unwrap();
    assert!(s.contains("Success"));
    let e = serde_json::to_string(&Status::Error).unwrap();
    assert!(e.contains("Error"));
}

// ── success / error constructors ─────────────────────────────────────────────

#[test]
fn success_sets_status_and_data() {
    let r: ApiResponse<u32> = ApiResponse::success(42_u32);
    assert_eq!(r.status, Status::Success);
    assert_eq!(r.data, Some(42));
    assert!(r.meta.is_none());
}

#[test]
fn success_message_is_default() {
    let r: ApiResponse<u32> = ApiResponse::success(1);
    assert_eq!(r.message, "Operation completed successfully");
}

#[test]
fn success_with_message_sets_custom_message() {
    let r: ApiResponse<&str> = ApiResponse::success_with_message("hello", "custom msg");
    assert_eq!(r.status, Status::Success);
    assert_eq!(r.message, "custom msg");
    assert_eq!(r.data, Some("hello"));
}

#[test]
fn success_with_meta_stores_meta() {
    let meta = ResponseMeta {
        pagination: None,
        request_id: None,
        timestamp: None,
        custom: None,
    };
    let r: ApiResponse<i32> = ApiResponse::success_with_meta(1, "ok", meta);
    assert_eq!(r.status, Status::Success);
    assert!(r.meta.is_some());
}

#[test]
fn error_constructor_sets_error_status() {
    let e = ApiResponse::<()>::error("oops");
    assert_eq!(e.status, Status::Error);
    assert_eq!(e.message, "oops");
    assert!(e.data.is_none());
    assert!(e.meta.is_none());
}

#[test]
fn error_with_meta_embeds_error_code() {
    let e = ApiResponse::<()>::error_with_meta("fail", "MY_CODE", None);
    assert_eq!(e.status, Status::Error);
    let meta = e.meta.unwrap();
    let custom = meta.custom.unwrap();
    assert_eq!(custom["error_code"], "MY_CODE");
}

#[test]
fn error_with_meta_includes_request_id_and_timestamp_by_default() {
    let e = ApiResponse::<()>::error_with_meta("fail", "CODE", None);
    let meta = e.meta.unwrap();
    assert!(meta.request_id.is_some());
    assert!(meta.timestamp.is_some());
}

#[test]
fn error_with_meta_uses_supplied_meta() {
    let supplied = ResponseMeta {
        pagination: None,
        request_id: None,
        timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        custom: Some(serde_json::json!({"error_code": "SUPPLIED"})),
    };
    let e = ApiResponse::<()>::error_with_meta("fail", "IGNORED", Some(supplied));
    let custom = e.meta.unwrap().custom.unwrap();
    assert_eq!(custom["error_code"], "SUPPLIED");
}

// ── paginated constructor ────────────────────────────────────────────────────

#[test]
fn paginated_computes_total_pages() {
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 100, 1, 20);
    let pag = r.meta.unwrap().pagination.unwrap();
    assert_eq!(pag.total, 100);
    assert_eq!(pag.page, 1);
    assert_eq!(pag.per_page, 20);
    assert_eq!(pag.total_pages, 5);
    assert!(!pag.has_previous);
    assert!(pag.has_next);
}

#[test]
fn paginated_last_page_has_no_next() {
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 100, 5, 20);
    let pag = r.meta.unwrap().pagination.unwrap();
    assert!(!pag.has_next);
    assert!(pag.has_previous);
}

#[test]
fn paginated_single_page() {
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 5, 1, 20);
    let pag = r.meta.unwrap().pagination.unwrap();
    assert_eq!(pag.total_pages, 1);
    assert!(!pag.has_previous);
    assert!(!pag.has_next);
}

#[test]
fn paginated_per_page_zero_yields_one_total_page() {
    // per_page <= 0 → total_pages = 1 (guard branch)
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 50, 1, 0);
    let pag = r.meta.unwrap().pagination.unwrap();
    assert_eq!(pag.total_pages, 1);
}

#[test]
fn paginated_includes_request_id_and_timestamp() {
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 1, 1, 10);
    let meta = r.meta.unwrap();
    assert!(meta.request_id.is_some());
    assert!(meta.timestamp.is_some());
}

#[test]
fn paginated_middle_page_has_both_prev_and_next() {
    let r: ApiResponse<Vec<u8>> = ApiResponse::paginated(vec![], 100, 3, 20);
    let pag = r.meta.unwrap().pagination.unwrap();
    assert!(pag.has_previous);
    assert!(pag.has_next);
}

// ── PaginationMeta default ───────────────────────────────────────────────────

#[test]
fn pagination_meta_default_is_zeroed() {
    let p = PaginationMeta::default();
    assert_eq!(p.total, 0);
    assert_eq!(p.page, 0);
    assert_eq!(p.per_page, 0);
    assert_eq!(p.total_pages, 0);
    assert!(!p.has_previous);
    assert!(!p.has_next);
}

// ── ValidationViolation struct ───────────────────────────────────────────────

#[test]
fn validation_violation_fields_are_accessible() {
    let v = ValidationViolation {
        field: "email".to_string(),
        message: "required".to_string(),
        code: Some("NOT_BLANK".to_string()),
    };
    assert_eq!(v.field, "email");
    assert_eq!(v.message, "required");
    assert_eq!(v.code.unwrap(), "NOT_BLANK");
}

#[test]
fn validation_violation_code_can_be_none() {
    let v = ValidationViolation {
        field: "name".to_string(),
        message: "too short".to_string(),
        code: None,
    };
    assert!(v.code.is_none());
}
