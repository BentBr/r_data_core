//! Tests for `ApiError` — Display formatting, `status_code`, and `error_response` delegation.
#![allow(clippy::unwrap_used)]
#![allow(clippy::future_not_send)]

use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::ResponseError;

use crate::response::ApiError;

/// Read a `HttpResponse` body to a `serde_json::Value`.
async fn body_json(resp: actix_web::HttpResponse) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Display formatting ────────────────────────────────────────────────────────

#[test]
fn display_not_found_contains_message() {
    assert!(ApiError::NotFound("Order".into())
        .to_string()
        .contains("Order"));
}

#[test]
fn display_internal_error_contains_message() {
    assert!(ApiError::InternalError("crash".into())
        .to_string()
        .contains("crash"));
}

#[test]
fn display_bad_request_contains_message() {
    assert!(ApiError::BadRequest("bad".into())
        .to_string()
        .contains("bad"));
}

#[test]
fn display_unauthorized_contains_message() {
    assert!(ApiError::Unauthorized("token".into())
        .to_string()
        .contains("token"));
}

#[test]
fn display_forbidden_contains_message() {
    assert!(ApiError::Forbidden("denied".into())
        .to_string()
        .contains("denied"));
}

#[test]
fn display_inactive_contains_message() {
    assert!(ApiError::Inactive("disabled".into())
        .to_string()
        .contains("disabled"));
}

#[test]
fn display_unprocessable_entity_contains_message() {
    assert!(ApiError::UnprocessableEntity("invalid".into())
        .to_string()
        .contains("invalid"));
}

// ── status_code mapping ───────────────────────────────────────────────────────

#[test]
fn status_code_not_found() {
    assert_eq!(
        ApiError::NotFound(String::new()).status_code(),
        StatusCode::NOT_FOUND
    );
}

#[test]
fn status_code_internal_error() {
    assert_eq!(
        ApiError::InternalError(String::new()).status_code(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn status_code_bad_request() {
    assert_eq!(
        ApiError::BadRequest(String::new()).status_code(),
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn status_code_unauthorized() {
    assert_eq!(
        ApiError::Unauthorized(String::new()).status_code(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn status_code_forbidden() {
    assert_eq!(
        ApiError::Forbidden(String::new()).status_code(),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn status_code_inactive_is_also_forbidden() {
    assert_eq!(
        ApiError::Inactive(String::new()).status_code(),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn status_code_unprocessable_entity() {
    assert_eq!(
        ApiError::UnprocessableEntity(String::new()).status_code(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

// ── error_response delegation ─────────────────────────────────────────────────

#[actix_web::test]
async fn error_response_not_found_delegates_to_api_response() {
    let resp = ApiError::NotFound("Invoice".into()).error_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Invoice not found"));
    assert_eq!(body["meta"]["custom"]["error_code"], "RESOURCE_NOT_FOUND");
}

#[actix_web::test]
async fn error_response_internal_error_delegates() {
    let resp = ApiError::InternalError("db gone".into()).error_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_json(resp).await;
    assert_eq!(
        body["meta"]["custom"]["error_code"],
        "INTERNAL_SERVER_ERROR"
    );
}

#[actix_web::test]
async fn error_response_bad_request_delegates() {
    let resp = ApiError::BadRequest("missing x".into()).error_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "BAD_REQUEST");
}

#[actix_web::test]
async fn error_response_unauthorized_delegates() {
    let resp = ApiError::Unauthorized("expired".into()).error_response();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "UNAUTHORIZED");
}

#[actix_web::test]
async fn error_response_forbidden_delegates() {
    let resp = ApiError::Forbidden("role mismatch".into()).error_response();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "FORBIDDEN");
}

#[actix_web::test]
async fn error_response_inactive_delegates() {
    let resp = ApiError::Inactive("account off".into()).error_response();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "RESOURCE_INACTIVE");
}

#[actix_web::test]
async fn error_response_unprocessable_entity_delegates() {
    let resp = ApiError::UnprocessableEntity("bad data".into()).error_response();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "VALIDATION_ERROR");
}
