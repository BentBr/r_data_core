use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub status: Status,
    pub message: String,
    pub data: Option<T>,
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
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            status: Status::Success,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            status: Status::Error,
            message: message.to_string(),
            data: None,
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

    /// Create a resource that was created successfully
    pub fn created<D: Serialize>(data: D) -> HttpResponse {
        let response = ApiResponse {
            status: Status::Success,
            message: "Resource created successfully".to_string(),
            data: Some(data),
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
        };
        response.to_http_response(StatusCode::OK)
    }

    pub fn created_message(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Success,
            message: message.to_string(),
            data: None,
        };
        response.to_http_response(StatusCode::CREATED)
    }

    pub fn not_found(resource: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: format!("{} not found", resource),
            data: None,
        };
        response.to_http_response(StatusCode::NOT_FOUND)
    }

    pub fn internal_error(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
        };
        response.to_http_response(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn bad_request(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
        };
        response.to_http_response(StatusCode::BAD_REQUEST)
    }

    pub fn unauthorized(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
        };
        log::debug!("Creating unauthorized response with message: {}", message);
        response.to_http_response(StatusCode::UNAUTHORIZED)
    }

    pub fn forbidden(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
        };
        response.to_http_response(StatusCode::FORBIDDEN)
    }

    pub fn inactive(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
        };
        response.to_http_response(StatusCode::FORBIDDEN)
    }
    pub fn unprocessable_entity(message: &str) -> HttpResponse {
        let response = Self {
            status: Status::Error,
            message: message.to_string(),
            data: None,
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
            ApiError::Inactive(msg) => ApiResponse::<()>::forbidden(msg),
            ApiError::UnprocessableEntity(msg) => ApiResponse::<()>::unprocessable_entity(msg),
        }
    }
}
