use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ResponseError,
    http::StatusCode,
    middleware::{ErrorHandlerResponse, ErrorHandlers as ActixErrorHandlers},
    web::Json,
    HttpResponse,
};
use serde::Serialize;
use serde_json::json;

use crate::api::response::ApiResponse;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

pub struct AppErrorHandlers;

impl AppErrorHandlers {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_error<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
        let request = res.request().clone();
        let error = res
            .response()
            .error()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());
        let status = res.response().status();

        // Create a new response
        let new_response = match status {
            StatusCode::UNAUTHORIZED => {
                let json = Json(json!({
                    "status": "Error",
                    "message": "Authentication required or invalid credentials",
                    "data": null
                }));
                HttpResponse::Unauthorized().json(json)
            }
            StatusCode::FORBIDDEN => {
                let json = Json(json!({
                    "status": "Error",
                    "message": "You don't have permission to access this resource",
                    "data": null
                }));
                HttpResponse::Forbidden().json(json)
            }
            StatusCode::NOT_FOUND => {
                let json = Json(json!({
                    "status": "Error",
                    "message": "API resource not found",
                    "data": null
                }));
                HttpResponse::NotFound().json(json)
            }
            status_code if status_code.is_client_error() => {
                let json = Json(json!({
                    "status": "Error",
                    "message": error,
                    "data": null
                }));
                HttpResponse::build(status_code).json(json)
            }
            status_code if status_code.is_server_error() => {
                // Log server errors
                log::error!("Server error: {} - {}", status_code, error);
                let json = Json(json!({
                    "status": "Error",
                    "message": "An internal server error occurred",
                    "data": null
                }));
                HttpResponse::InternalServerError().json(json)
            }
            _ => {
                let json = Json(json!({
                    "status": "Error",
                    "message": error,
                    "data": null
                }));
                HttpResponse::build(status).json(json)
            }
        };

        let (request, _) = res.into_parts();
        let response = ServiceResponse::new(request, new_response.map_into_right_body());
        Ok(ErrorHandlerResponse::Response(response))
    }

    pub fn custom_error_response<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
        Self::handle_error(res)
    }
}

// Create and configure Actix Web error handlers
pub fn create_error_handlers() -> ActixErrorHandlers<BoxBody> {
    ActixErrorHandlers::new()
        .handler(StatusCode::NOT_FOUND, AppErrorHandlers::handle_error)
        .handler(StatusCode::UNAUTHORIZED, AppErrorHandlers::handle_error)
        .handler(StatusCode::FORBIDDEN, AppErrorHandlers::handle_error)
        .handler(
            StatusCode::INTERNAL_SERVER_ERROR,
            AppErrorHandlers::handle_error,
        )
        .handler(StatusCode::BAD_REQUEST, AppErrorHandlers::handle_error)
}

// Add a custom panic handler for middleware errors
pub fn handle_middleware_panic(err: &actix_web::Error) -> HttpResponse {
    log::error!("Middleware panic: {:?}", err);

    // Return a friendly error response
    HttpResponse::InternalServerError().json(json!({
        "status": "Error",
        "message": "An unexpected error occurred processing your request",
        "data": null
    }))
}

// Type alias to make the code more readable
type Result<T> = std::result::Result<T, actix_web::Error>;
