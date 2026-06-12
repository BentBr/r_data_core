//! Tests for `ApiResponse` HTTP helper methods (status codes + JSON bodies).
#![allow(clippy::unwrap_used)]
#![allow(clippy::future_not_send)]

use actix_web::body::to_bytes;
use actix_web::http::StatusCode;

use crate::response::{ApiResponse, ValidationViolation};

/// Read a `HttpResponse` body to a `serde_json::Value`.
async fn body_json(resp: actix_web::HttpResponse) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── to_http_response ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn to_http_response_uses_given_status_code() {
    let r: ApiResponse<u32> = ApiResponse::success(7);
    let http = r.to_http_response(StatusCode::ACCEPTED);
    assert_eq!(http.status(), StatusCode::ACCEPTED);
}

// ── ok / ok_with_message / ok_paginated ──────────────────────────────────────

#[actix_web::test]
async fn ok_returns_200_with_data() {
    let resp = ApiResponse::ok(42_u32);
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "Success");
    assert_eq!(body["data"], 42);
}

#[actix_web::test]
async fn ok_with_message_returns_200_with_custom_message() {
    let resp = ApiResponse::ok_with_message(1_u32, "done");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["message"], "done");
}

#[actix_web::test]
async fn ok_paginated_returns_200_with_pagination_meta() {
    let resp = ApiResponse::ok_paginated(vec![1_u32, 2], 10, 1, 5);
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["pagination"]["total"], 10);
    assert_eq!(body["meta"]["pagination"]["per_page"], 5);
}

// ── created ──────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn created_returns_201() {
    let resp = ApiResponse::<u32>::created(99_u32);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["message"], "Resource created successfully");
    assert_eq!(body["status"], "Success");
}

// ── message / created_message ─────────────────────────────────────────────────

#[actix_web::test]
async fn message_helper_returns_200_with_no_data() {
    let resp = ApiResponse::<()>::message("pong");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["message"], "pong");
    assert_eq!(body["status"], "Success");
    assert!(body["data"].is_null());
}

#[actix_web::test]
async fn created_message_helper_returns_201() {
    let resp = ApiResponse::<()>::created_message("added");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "Success");
    assert_eq!(body["message"], "added");
}

// ── error status helpers ──────────────────────────────────────────────────────

#[actix_web::test]
async fn not_found_returns_404_with_error_code() {
    let resp = ApiResponse::<()>::not_found("User");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "Error");
    assert!(body["message"].as_str().unwrap().contains("User"));
    assert_eq!(body["meta"]["custom"]["error_code"], "RESOURCE_NOT_FOUND");
}

#[actix_web::test]
async fn not_found_message_includes_resource_name() {
    let resp = ApiResponse::<()>::not_found("Invoice");
    let body = body_json(resp).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Invoice not found"));
}

#[actix_web::test]
async fn conflict_returns_409_with_error_code() {
    let resp = ApiResponse::<()>::conflict("duplicate key");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "Error");
    assert_eq!(body["meta"]["custom"]["error_code"], "RESOURCE_CONFLICT");
}

#[actix_web::test]
async fn bad_request_returns_400_with_error_code() {
    let resp = ApiResponse::<()>::bad_request("missing field");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "Error");
    assert_eq!(body["meta"]["custom"]["error_code"], "BAD_REQUEST");
}

#[actix_web::test]
async fn unauthorized_returns_401_with_error_code() {
    let resp = ApiResponse::<()>::unauthorized("invalid token");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "UNAUTHORIZED");
}

#[actix_web::test]
async fn forbidden_returns_403_with_error_code() {
    let resp = ApiResponse::<()>::forbidden("no access");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "FORBIDDEN");
}

#[actix_web::test]
async fn inactive_returns_403_with_inactive_code() {
    let resp = ApiResponse::<()>::inactive("account disabled");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "RESOURCE_INACTIVE");
}

#[actix_web::test]
async fn too_many_requests_returns_429_with_error_code() {
    let resp = ApiResponse::<()>::too_many_requests("slow down");
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "TOO_MANY_REQUESTS");
}

#[actix_web::test]
async fn internal_error_returns_500_with_error_code() {
    let resp = ApiResponse::<()>::internal_error("db failure");
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_json(resp).await;
    assert_eq!(
        body["meta"]["custom"]["error_code"],
        "INTERNAL_SERVER_ERROR"
    );
}

#[actix_web::test]
async fn unprocessable_entity_returns_422_with_error_code() {
    let resp = ApiResponse::<()>::unprocessable_entity("bad input");
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["meta"]["custom"]["error_code"], "VALIDATION_ERROR");
}

// ── unprocessable_entity_with_violations ─────────────────────────────────────

#[actix_web::test]
async fn unprocessable_entity_with_violations_returns_422() {
    let violations = vec![
        ValidationViolation {
            field: "email".to_string(),
            message: "must not be blank".to_string(),
            code: Some("NOT_BLANK".to_string()),
        },
        ValidationViolation {
            field: "name".to_string(),
            message: "too short".to_string(),
            code: None,
        },
    ];
    let resp =
        ApiResponse::<()>::unprocessable_entity_with_violations("validation failed", violations);
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["message"], "validation failed");
    assert_eq!(body["violations"][0]["field"], "email");
    assert_eq!(body["violations"][0]["code"], "NOT_BLANK");
    assert!(body["violations"][1]["code"].is_null());
}

#[actix_web::test]
async fn unprocessable_entity_with_violations_empty_list() {
    let resp = ApiResponse::<()>::unprocessable_entity_with_violations("no detail", vec![]);
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert!(body["violations"].as_array().unwrap().is_empty());
}

// ── error helpers include request_id and timestamp ───────────────────────────

#[actix_web::test]
async fn error_helpers_include_meta_with_timestamp_and_request_id() {
    // Spot-check one representative error helper
    let resp = ApiResponse::<()>::bad_request("x");
    let body = body_json(resp).await;
    assert!(!body["meta"]["request_id"].is_null());
    assert!(!body["meta"]["timestamp"].is_null());
}
