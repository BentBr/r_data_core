use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, middleware::ErrorHandlerResponse, Result};
use serde_json;

use crate::api::response::ApiResponse;

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
            let response = ApiResponse::<()>::internal_error();
            let body = serde_json::to_string(&response).unwrap();
            
            let resp = HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(body);
                
            return Ok(ErrorHandlerResponse::Response(
                ServiceResponse::new(res.request().clone(), resp.map_into_right_body())
            ));
        }

        if status_code == StatusCode::NOT_FOUND {
            let response = ApiResponse::<()>::not_found("Resource not found");
            let body = serde_json::to_string(&response).unwrap();
            
            let resp = HttpResponse::NotFound()
                .content_type("application/json")
                .body(body);
                
            return Ok(ErrorHandlerResponse::Response(
                ServiceResponse::new(res.request().clone(), resp.map_into_right_body())
            ));
        }

        Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
    }
} 