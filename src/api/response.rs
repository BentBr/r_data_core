use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    pub fn paginated(data: T, total: i64, page: i64, per_page: i64) -> Self {
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

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            status: Status::Error,
            message: message.to_string(),
            data: None,
            meta: None,
        }
    }

    /// Create an error response with error code and metadata
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
    pub fn message(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Success,
            message: message.to_string(),
            data: None,
            meta: None,
        };
        response.to_http_response(StatusCode::OK)
    }

    pub fn created_message(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Success,
            message: message.to_string(),
            data: None,
            meta: None,
        };
        response.to_http_response(StatusCode::CREATED)
    }

    pub fn not_found(resource: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: format!("{} not found", resource),
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
        log::debug!("Creating unauthorized response with message: {}", message);
        response.to_http_response(StatusCode::UNAUTHORIZED)
    }

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
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::Inactive(msg) => write!(f, "Inactive: {}", msg),
            ApiError::UnprocessableEntity(msg) => write!(f, "Unprocessable entity: {}", msg),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Inactive(_) => StatusCode::FORBIDDEN,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(resource) => ApiResponse::<()>::not_found(resource),
            ApiError::InternalError(msg) => ApiResponse::<()>::internal_error(msg),
            ApiError::BadRequest(msg) => ApiResponse::<()>::bad_request(msg),
            ApiError::Unauthorized(msg) => ApiResponse::<()>::unauthorized(msg),
            ApiError::Forbidden(msg) => ApiResponse::<()>::forbidden(msg),
            ApiError::Inactive(msg) => ApiResponse::<()>::inactive(msg),
            ApiError::UnprocessableEntity(msg) => ApiResponse::<()>::unprocessable_entity(msg),
        }
    }
}
