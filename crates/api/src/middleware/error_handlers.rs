use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    http::StatusCode,
    middleware::{ErrorHandlerResponse, ErrorHandlers as ActixErrorHandlers},
    HttpResponse,
};

use crate::response::ApiResponse;

pub struct AppErrorHandlers;

impl AppErrorHandlers {
    pub fn handle_error<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
        let status = res.response().status();

        // Extract the original error message
        let error = res
            .response()
            .error()
            .map(|e| e.to_string())
            .unwrap_or_else(|| {
                // For unauthorized responses without an error, use a specific message
                if status == StatusCode::UNAUTHORIZED {
                    "Invalid credentials".to_string()
                } else {
                    "Unknown error".to_string()
                }
            });

        log::debug!("Error handler called for status {}: {}", status, error);

        // Check if this is a deserialization error - be more flexible with the detection
        let is_deserialization_error = error.contains("invalid type: string")
            || error.contains("expected i64")
            || error.contains("Query deserialize error");

        // Create a new response using ApiResponse
        let new_response = match status {
            StatusCode::UNAUTHORIZED => ApiResponse::<()>::unauthorized(&error),
            StatusCode::FORBIDDEN => {
                ApiResponse::<()>::forbidden("You don't have permission to access this resource")
            }
            StatusCode::NOT_FOUND => ApiResponse::<()>::not_found("API resource not found"),
            StatusCode::BAD_REQUEST if is_deserialization_error => {
                // Handle deserialization errors specifically
                let response = ApiResponse::<()>::error_with_meta(
                    &error,
                    "DESERIALIZATION_ERROR",
                    None,
                );
                HttpResponse::build(StatusCode::BAD_REQUEST).json(response)
            }
            status_code if status_code.is_client_error() => ApiResponse::<()>::bad_request(&error),
            status_code if status_code.is_server_error() => {
                // Log server errors
                log::error!("Server error: {} - {}", status_code, error);
                ApiResponse::<()>::internal_error("An internal server error occurred")
            }
            _ => {
                // Create a custom response with the appropriate status code
                let response = ApiResponse::<()>::error(&error);
                HttpResponse::build(status).json(response)
            }
        };

        let (request, _) = res.into_parts();
        let response = ServiceResponse::new(request, new_response.map_into_right_body());
        Ok(ErrorHandlerResponse::Response(response))
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
#[allow(dead_code)] // Handler for future use
pub fn handle_middleware_panic(err: &actix_web::Error) -> HttpResponse {
    log::error!("Middleware panic: {:?}", err);

    // Return a friendly error response
    ApiResponse::<()>::internal_error("An unexpected error occurred processing your request")
}

// Type alias to make the code more readable
type Result<T> = std::result::Result<T, actix_web::Error>;
