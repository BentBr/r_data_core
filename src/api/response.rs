use serde::{Deserialize, Serialize};
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Status {
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> 
where 
    T: Serialize
{
    pub status: Status,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> 
where 
    T: Serialize
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
}

// Default responses
impl ApiResponse<()> {
    pub fn not_found(resource: &str) -> Self {
        Self {
            status: Status::Error,
            message: format!("{} not found", resource),
            data: None,
        }
    }

    pub fn internal_error() -> Self {
        Self {
            status: Status::Error,
            message: "Internal server error".to_string(),
            data: None,
        }
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
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
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
        }
    }

    fn error_response(&self) -> HttpResponse {
        let response = match self {
            ApiError::NotFound(msg) => ApiResponse::<()>::error(msg),
            ApiError::InternalError(_) => ApiResponse::<()>::internal_error(),
            ApiError::BadRequest(msg) => ApiResponse::<()>::error(msg),
            ApiError::Unauthorized(msg) => ApiResponse::<()>::error(msg),
            ApiError::Forbidden(msg) => ApiResponse::<()>::error(msg),
        };

        HttpResponse::build(self.status_code()).json(response)
    }
}
