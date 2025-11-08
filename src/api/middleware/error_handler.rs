use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
    Error, HttpResponse,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use log::{error, warn};
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
        // Capture request context for logging
        let method = req.method().clone();
        let path = req.path().to_string();
        let request_id = Uuid::now_v7();

        let fut = self.service.call(req);

        Box::pin(async move {
            match fut.await {
                Ok(response) => {
                    let status_code = response.status();
                    if status_code.is_server_error() {
                        error!(
                            target: "http",
                            "HTTP {} {} -> {} (request_id={})",
                            method,
                            path,
                            status_code.as_u16(),
                            request_id
                        );
                    } else if status_code.is_client_error() {
                        warn!(
                            target: "http",
                            "HTTP {} {} -> {} (request_id={})",
                            method,
                            path,
                            status_code.as_u16(),
                            request_id
                        );
                    }
                    Ok(response)
                }
                Err(err) => {
                    // Log the error with context
                    error!(
                        target: "http",
                        "HTTP {} {} -> 500 error: {} (request_id={})",
                        method,
                        path,
                        err,
                        request_id
                    );
                    let response = handle_error(&err, Some(request_id));
                    Err(actix_web::error::InternalError::from_response("", response).into())
                }
            }
        })
    }
}

/// Convert any error to our standardized API response format
fn handle_error(err: &Error, req_id: Option<Uuid>) -> HttpResponse {
    // Get error message
    let error_message = err.to_string();

    // Default to internal server error
    let status_code = StatusCode::INTERNAL_SERVER_ERROR;
    let error_code = "INTERNAL_SERVER_ERROR";

    let response = ApiResponse {
        status: Status::Error,
        message: error_message,
        data: None as Option<()>,
        meta: Some(ResponseMeta {
            pagination: None,
            request_id: req_id.or(Some(Uuid::now_v7())),
            timestamp: Some(time::OffsetDateTime::now_utc().to_string()),
            custom: Some(serde_json::json!({"error_code": error_code})),
        }),
    };

    HttpResponse::build(status_code).json(response)
}
