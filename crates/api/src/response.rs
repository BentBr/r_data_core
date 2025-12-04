use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

/// Individual validation violation for Symfony-style errors
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationViolation {
    /// The field that has the validation error
    pub field: String,
    /// The error message for this field
    pub message: String,
    /// Optional error code (e.g., `"NOT_BLANK"`, `"NOT_NULL"`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Validation error response in Symfony format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationErrorResponse {
    /// Overall error message
    pub message: String,
    /// List of validation violations
    pub violations: Vec<ValidationViolation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Status {
    Success,
    Error,
}

/// Metadata for paginated responses
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PaginationMeta {
    /// Total number of items available
    pub total: i64,
    /// Current page number
    pub page: i64,
    /// Items per page
    pub per_page: i64,
    /// Total number of pages
    pub total_pages: i64,
    /// If there is a previous page
    pub has_previous: bool,
    /// If there is a next page
    pub has_next: bool,
}

/// Metadata for API responses
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ResponseMeta {
    /// Pagination information (if applicable)
    pub pagination: Option<PaginationMeta>,
    /// Request UUID for tracking
    pub request_id: Option<Uuid>,
    /// Timestamp of the response
    pub timestamp: Option<String>,
    /// Additional custom metadata
    pub custom: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    /// Response status (success/error)
    pub status: Status,
    /// Human-readable message
    pub message: String,
    /// Response data payload
    pub data: Option<T>,
    /// Additional metadata
    pub meta: Option<ResponseMeta>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            status: Status::Success,
            message: "Operation completed successfully".to_string(),
            data: Some(data),
            meta: None,
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            status: Status::Success,
            message: message.to_string(),
            data: Some(data),
            meta: None,
        }
    }

    /// Create a success response with metadata
    pub fn success_with_meta(data: T, message: &str, meta: ResponseMeta) -> Self {
        Self {
            status: Status::Success,
            message: message.to_string(),
            data: Some(data),
            meta: Some(meta),
        }
    }

    /// Create a paginated success response
    ///
    /// # Panics
    /// This function may panic if `per_page` is 0 (division by zero)
    pub fn paginated(data: T, total: i64, page: i64, per_page: i64) -> Self {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;

        let pagination = PaginationMeta {
            total,
            page,
            per_page,
            total_pages,
            has_previous: page > 1,
            has_next: page < total_pages,
        };

        let meta = ResponseMeta {
            pagination: Some(pagination),
            request_id: Some(Uuid::now_v7()),
            timestamp: Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
            custom: None,
        };

        Self {
            status: Status::Success,
            message: "Operation completed successfully".to_string(),
            data: Some(data),
            meta: Some(meta),
        }
    }

    #[must_use]
    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: None,
        }
    }

    /// Create an error response with error code and metadata
    ///
    /// # Panics
    /// This function may panic if JSON serialization fails
    #[must_use]
    pub fn error_with_meta(
        message: &str,
        error_code: &str,
        meta: Option<ResponseMeta>,
    ) -> ApiResponse<()> {
        let custom = serde_json::json!({
            "error_code": error_code
        });

        let meta = meta.unwrap_or_else(|| ResponseMeta {
            pagination: None,
            request_id: Some(Uuid::now_v7()),
            timestamp: Some(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
            custom: Some(custom),
        });

        ApiResponse {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: Some(meta),
        }
    }

    pub fn to_http_response(&self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code).json(self)
    }

    // HTTP response helpers
    pub fn ok(data: T) -> HttpResponse {
        let response = Self::success(data);
        response.to_http_response(StatusCode::OK)
    }

    pub fn ok_with_message(data: T, message: &str) -> HttpResponse {
        let response = Self::success_with_message(data, message);
        response.to_http_response(StatusCode::OK)
    }

    /// Return a paginated response
    pub fn ok_paginated(data: T, total: i64, page: i64, per_page: i64) -> HttpResponse {
        let response = Self::paginated(data, total, page, per_page);
        response.to_http_response(StatusCode::OK)
    }

    /// Create a resource that was created successfully
    pub fn created<D: Serialize>(data: D) -> HttpResponse {
        let response = ApiResponse {
            status: Status::Success,
            message: "Resource created successfully".to_string(),
            data: Some(data),
            meta: None,
        };

        response.to_http_response(StatusCode::CREATED)
    }
}

// Default responses
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

// Custom error type that implements ResponseError
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    InternalError(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Inactive(String),
    UnprocessableEntity(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::InternalError(msg) => write!(f, "Internal error: {msg}"),
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            Self::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
            Self::Inactive(msg) => write!(f, "Inactive: {msg}"),
            Self::UnprocessableEntity(msg) => write!(f, "Unprocessable entity: {msg}"),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) | Self::Inactive(_) => StatusCode::FORBIDDEN,
            Self::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::NotFound(resource) => ApiResponse::<()>::not_found(resource),
            Self::InternalError(msg) => ApiResponse::<()>::internal_error(msg),
            Self::BadRequest(msg) => ApiResponse::<()>::bad_request(msg),
            Self::Unauthorized(msg) => ApiResponse::<()>::unauthorized(msg),
            Self::Forbidden(msg) => ApiResponse::<()>::forbidden(msg),
            Self::Inactive(msg) => ApiResponse::<()>::inactive(msg),
            Self::UnprocessableEntity(msg) => ApiResponse::<()>::unprocessable_entity(msg),
        }
    }
}
