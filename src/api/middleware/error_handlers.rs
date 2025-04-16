use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::{middleware::ErrorHandlerResponse, HttpResponse, Result};
use serde::Serialize;
use serde_json;

use crate::api::response::ApiResponse;

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
}

pub struct ErrorHandlers;

impl ErrorHandlers {
    pub fn new() -> Self {
        ErrorHandlers {}
    }

    pub fn handle_error<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>>
    where
        B: 'static,
    {
        let status_code = res.status();

        if status_code == StatusCode::INTERNAL_SERVER_ERROR {
            let error_response = ErrorResponse {
                success: false,
                message: "An internal server error occurred".to_string(),
            };
            let body = serde_json::to_string(&error_response).unwrap();

            let resp = HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(body);

            return Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
                res.request().clone(),
                resp.map_into_right_body(),
            )));
        }

        if status_code == StatusCode::NOT_FOUND {
            let error_response = ErrorResponse {
                success: false,
                message: "Resource not found".to_string(),
            };
            let body = serde_json::to_string(&error_response).unwrap();

            let resp = HttpResponse::NotFound()
                .content_type("application/json")
                .body(body);

            return Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
                res.request().clone(),
                resp.map_into_right_body(),
            )));
        }

        Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
    }
}
