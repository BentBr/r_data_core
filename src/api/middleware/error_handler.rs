use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
    Error, HttpResponse,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use uuid::Uuid;

use crate::api::response::{ApiResponse, ResponseMeta, Status};

/// Error handler middleware to ensure all responses follow our API standards
pub struct ErrorHandler;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandler
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ErrorHandlerMiddleware { service })
    }
}

pub struct ErrorHandlerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ErrorHandlerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            match fut.await {
                Ok(response) => {
                    // If the response is an error, ensure it follows our standards
                    let status_code = response.status();
                    if status_code.is_client_error() || status_code.is_server_error() {
                        // Check if the response body is already in our standard format
                        // If not, transform it

                        // For simplicity, we'll pass through the response for now
                        // In a real implementation, you would inspect and potentially transform the body
                        Ok(response)
                    } else {
                        Ok(response)
                    }
                }
                Err(err) => {
                    let response = handle_error(&err);
                    Err(actix_web::error::InternalError::from_response("", response).into())
                }
            }
        })
    }
}

/// Convert any error to our standardized API response format
fn handle_error(err: &Error) -> HttpResponse {
    // Get error message
    let error_message = err.to_string();

    // Default to internal server error
    let status_code = StatusCode::INTERNAL_SERVER_ERROR;
    let error_code = "INTERNAL_SERVER_ERROR";

    let meta = ResponseMeta {
        pagination: None,
        request_id: Some(Uuid::now_v7()),
        timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
        custom: Some(serde_json::json!({"error_code": error_code})),
    };

    let response = ApiResponse {
        status: Status::Error,
        message: error_message,
        data: None as Option<()>,
        meta: Some(meta),
    };

    HttpResponse::build(status_code).json(response)
}
