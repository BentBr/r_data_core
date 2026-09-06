use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

/// Individual validation violation for Symfony-style errors
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct ValidationViolation {
    /// The field that has the validation error
    pub field: String,
    /// The error message for this field
    pub message: String,
    /// Optional error code (e.g., `"NOT_BLANK"`, `"NOT_NULL"`)
    pub code: Option<String>,
    /// JSON path to the offending value, e.g. `steps[0].to.type`.
    ///
    /// More precise than `field` where the producer can manage it; `None` when
    /// it cannot, which is honest rather than pointing somewhere wrong. Always
    /// serialized (as `null` when absent) so the wire format matches the
    /// generated TypeScript — ts-rs cannot read `skip_serializing_if`.
    #[serde(default)]
    pub json_path: Option<String>,
    /// Alternatives the caller may use instead, where the failure is a closed
    /// set — an unknown enum variant, say. Empty means "not enumerable", never
    /// "nothing is legal".
    #[serde(default)]
    pub legal_values: Vec<String>,
}

impl ValidationViolation {
    /// A violation located by field name, with no JSON path and no enumerable
    /// alternatives — the shape almost every producer needs.
    ///
    /// Set `json_path` and `legal_values` afterwards where they can genuinely
    /// be supplied; the DSL diagnostics do, because a caller fixing a nested
    /// step needs the exact key rather than the top-level field.
    #[must_use]
    pub fn field(field: impl Into<String>, message: impl Into<String>, code: &str) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: Some(code.to_string()),
            json_path: None,
            legal_values: Vec::new(),
        }
    }
}

/// Validation error response in Symfony format
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct ValidationErrorResponse {
    /// Overall error message
    pub message: String,
    /// List of validation violations
    pub violations: Vec<ValidationViolation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Status {
    Success,
    Error,
}

/// Metadata for paginated responses
#[derive(Debug, Serialize, Deserialize, Clone, Default, TS)]
#[ts(export)]
pub struct PaginationMeta {
    /// Total number of items available
    #[ts(type = "number")]
    pub total: i64,
    /// Current page number
    #[ts(type = "number")]
    pub page: i64,
    /// Items per page
    #[ts(type = "number")]
    pub per_page: i64,
    /// Total number of pages
    #[ts(type = "number")]
    pub total_pages: i64,
    /// If there is a previous page
    pub has_previous: bool,
    /// If there is a next page
    pub has_next: bool,
}

/// Metadata for API responses
#[derive(Debug, Serialize, Deserialize, Clone, Default, TS)]
#[ts(export)]
pub struct ResponseMeta {
    /// Pagination information (if applicable)
    pub pagination: Option<PaginationMeta>,
    /// Request UUID for tracking
    #[ts(type = "string | null")]
    pub request_id: Option<Uuid>,
    /// Timestamp of the response
    pub timestamp: Option<String>,
    /// Additional custom metadata
    #[ts(type = "unknown")]
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
    /// May panic if `total + per_page - 1` overflows with large values
    pub fn paginated(data: T, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = if per_page <= 0 {
            1
        } else {
            total.saturating_add(per_page - 1) / per_page
        };

        let pagination = PaginationMeta {
            total,
            page,
            per_page,
            total_pages,
            has_previous: page > 1,
            has_next: page < total_pages,
        };

        // Rfc3339 formatting of a UTC datetime is infallible
        #[allow(clippy::unwrap_used)]
        let timestamp = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
        let meta = ResponseMeta {
            pagination: Some(pagination),
            request_id: Some(Uuid::now_v7()),
            timestamp: Some(timestamp),
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

        // Rfc3339 formatting of a UTC datetime is infallible
        #[allow(clippy::unwrap_used)]
        let timestamp = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
        let meta = meta.unwrap_or_else(|| ResponseMeta {
            pagination: None,
            request_id: Some(Uuid::now_v7()),
            timestamp: Some(timestamp),
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

#[cfg(test)]
mod tests;
