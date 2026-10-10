#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The status-code constructors.
//!
//! Split from the response types themselves because they are a different kind
//! of thing: `mod.rs` defines what a response *is*, and this defines the
//! handful of failures the API knows how to report. Each carries an
//! `error_code` in its metadata, so a client can branch on the failure without
//! parsing the human-readable message.

use actix_web::{http::StatusCode, HttpResponse};
use uuid::Uuid;

use super::{ApiResponse, ResponseMeta, Status, ValidationErrorResponse, ValidationViolation};

impl ApiResponse<()> {
    #[must_use]
    pub fn message(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Success,
            message: message.to_string(),
            data: None,
            meta: None,
        };
        response.to_http_response(StatusCode::OK)
    }

    #[must_use]
    pub fn created_message(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Success,
            message: message.to_string(),
            data: None,
            meta: None,
        };
        response.to_http_response(StatusCode::CREATED)
    }

    #[must_use]
    pub fn not_found(resource: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: format!("{resource} not found"),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "RESOURCE_NOT_FOUND"})),
            }),
        };
        response.to_http_response(StatusCode::NOT_FOUND)
    }

    #[must_use]
    pub fn conflict(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "RESOURCE_CONFLICT"})),
            }),
        };
        response.to_http_response(StatusCode::CONFLICT)
    }

    #[must_use]
    pub fn internal_error(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "INTERNAL_SERVER_ERROR"})),
            }),
        };
        response.to_http_response(StatusCode::INTERNAL_SERVER_ERROR)
    }

    #[must_use]
    pub fn bad_request(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "BAD_REQUEST"})),
            }),
        };
        response.to_http_response(StatusCode::BAD_REQUEST)
    }

    #[must_use]
    pub fn unauthorized(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "UNAUTHORIZED"})),
            }),
        };
        log::debug!("Creating unauthorized response with message: {message}");
        response.to_http_response(StatusCode::UNAUTHORIZED)
    }

    #[must_use]
    pub fn forbidden(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "FORBIDDEN"})),
            }),
        };
        response.to_http_response(StatusCode::FORBIDDEN)
    }

    #[must_use]
    pub fn inactive(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "RESOURCE_INACTIVE"})),
            }),
        };
        response.to_http_response(StatusCode::FORBIDDEN)
    }

    #[must_use]
    pub fn too_many_requests(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "TOO_MANY_REQUESTS"})),
            }),
        };
        response.to_http_response(StatusCode::TOO_MANY_REQUESTS)
    }

    /// The request was fine; something this server depends on is not.
    ///
    /// Distinct from a 500 because it tells a caller the request is worth
    /// retrying, and distinct from a 403 because it points whoever is on call
    /// at infrastructure rather than at permissions.
    #[must_use]
    pub fn service_unavailable(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "SERVICE_UNAVAILABLE"})),
            }),
        };
        response.to_http_response(StatusCode::SERVICE_UNAVAILABLE)
    }

    #[must_use]
    pub fn unprocessable_entity(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(ResponseMeta {
                pagination: None,
                request_id: Some(Uuid::now_v7()),
                timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
                custom: Some(serde_json::json!({"error_code": "VALIDATION_ERROR"})),
            }),
        };
        response.to_http_response(StatusCode::UNPROCESSABLE_ENTITY)
    }

    /// Create a validation error response with field-specific violations (Symfony-style)
    #[must_use]
    pub fn unprocessable_entity_with_violations(
        message: &str,
        violations: Vec<ValidationViolation>,
    ) -> HttpResponse {
        let validation_response = ValidationErrorResponse {
            message: message.to_string(),
            violations,
        };
        HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY).json(validation_response)
    }
}
