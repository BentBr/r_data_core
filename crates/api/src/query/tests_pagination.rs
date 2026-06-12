#![allow(clippy::unwrap_used)]

use super::*;

// ── PaginationQuery::validate ─────────────────────────────────────────────────

#[test]
fn validate_ok_all_none() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: None,
        offset: None,
    };
    assert!(q.validate(100, false).is_ok());
}

#[test]
fn validate_page_zero_fails() {
    let q = PaginationQuery {
        page: Some(0),
        per_page: None,
        limit: None,
        offset: None,
    };
    let err = q.validate(100, false).unwrap_err();
    assert!(err.contains("page"), "expected 'page' in: {err}");
}

#[test]
fn validate_page_negative_fails() {
    let q = PaginationQuery {
        page: Some(-5),
        per_page: None,
        limit: None,
        offset: None,
    };
    assert!(q.validate(100, false).is_err());
}

#[test]
fn validate_per_page_minus_one_allowed() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(-1),
        limit: None,
        offset: None,
    };
    assert!(q.validate(100, true).is_ok());
}

#[test]
fn validate_per_page_minus_one_disallowed() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(-1),
        limit: None,
        offset: None,
    };
    let err = q.validate(100, false).unwrap_err();
    assert!(err.contains("per_page"), "expected 'per_page' in: {err}");
}

#[test]
fn validate_per_page_out_of_range_fails() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(200),
        limit: None,
        offset: None,
    };
    assert!(q.validate(100, false).is_err());
}

#[test]
fn validate_per_page_zero_fails() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(0),
        limit: None,
        offset: None,
    };
    assert!(q.validate(100, false).is_err());
}

#[test]
fn validate_limit_zero_fails() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(0),
        offset: None,
    };
    let err = q.validate(100, false).unwrap_err();
    assert!(err.contains("limit"), "expected 'limit' in: {err}");
}

#[test]
fn validate_limit_exceeds_max_fails() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(500),
        offset: None,
    };
    assert!(q.validate(100, false).is_err());
}

#[test]
fn validate_offset_negative_fails() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: None,
        offset: Some(-1),
    };
    let err = q.validate(100, false).unwrap_err();
    assert!(err.contains("offset"), "expected 'offset' in: {err}");
}

// ── PaginationQuery::to_limit_offset ─────────────────────────────────────────

#[test]
fn to_limit_offset_limit_only() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(30),
        offset: None,
    };
    let (limit, offset) = q.to_limit_offset(20, 100);
    assert_eq!(limit, 30);
    assert_eq!(offset, 0);
}

#[test]
fn to_limit_offset_per_page_only() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(15),
        limit: None,
        offset: None,
    };
    let (limit, offset) = q.to_limit_offset(20, 100);
    assert_eq!(limit, 15);
    assert_eq!(offset, 0);
}

#[test]
fn to_limit_offset_offset_only_uses_default() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: None,
        offset: Some(40),
    };
    let (limit, offset) = q.to_limit_offset(20, 100);
    assert_eq!(limit, 20);
    assert_eq!(offset, 40);
}

#[test]
fn to_limit_offset_unlimited_via_per_page_minus_one() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(-1),
        limit: None,
        offset: None,
    };
    let (limit, offset) = q.to_limit_offset(20, 100);
    assert_eq!(limit, -1);
    assert_eq!(offset, 0);
}

#[test]
fn to_limit_offset_page_and_per_page_unlimited() {
    let q = PaginationQuery {
        page: Some(3),
        per_page: Some(-1),
        limit: None,
        offset: None,
    };
    let (limit, offset) = q.to_limit_offset(20, 100);
    assert_eq!(limit, -1);
    assert_eq!(offset, 0);
}

// ── PaginationQuery::get_page ─────────────────────────────────────────────────

#[test]
fn get_page_limit_only_returns_page_one() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(20),
        offset: None,
    };
    assert_eq!(q.get_page(1), 1);
}

#[test]
fn get_page_derived_from_limit_and_offset() {
    // offset=40, limit=20 → page (40/20)+1 = 3
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(20),
        offset: Some(40),
    };
    assert_eq!(q.get_page(1), 3);
}

// ── PaginationQuery::get_per_page ─────────────────────────────────────────────

#[test]
fn get_per_page_unlimited_via_limit_minus_one() {
    let q = PaginationQuery {
        page: None,
        per_page: None,
        limit: Some(-1),
        offset: None,
    };
    assert_eq!(q.get_per_page(20, 100), -1);
}

#[test]
fn get_per_page_unlimited_via_per_page_minus_one() {
    let q = PaginationQuery {
        page: None,
        per_page: Some(-1),
        limit: None,
        offset: None,
    };
    assert_eq!(q.get_per_page(20, 100), -1);
}
