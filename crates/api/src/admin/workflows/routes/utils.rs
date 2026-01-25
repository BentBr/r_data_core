#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::HttpResponse;
use log::error;
use serde_json::Value;

use crate::response::ApiResponse;
use r_data_core_core::error::Error;

/// Check if a workflow config has from.api source type (accepts POST, cron disabled)
/// or to.format.output.mode === 'api' (exports via GET, cron disabled)
pub(crate) fn check_has_api_endpoint(config: &Value) -> bool {
    if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
        for step in steps {
            // Check for from.api source type (accepts POST, cron disabled)
            if let Some(from) = step.get("from") {
                if let Some(source) = from
                    .get("source")
                    .or_else(|| from.get("format").and_then(|f| f.get("source")))
                {
                    if let Some(source_type) = source.get("source_type").and_then(|v| v.as_str()) {
                        if source_type == "api" {
                            // from.api without endpoint field = accepts POST
                            if let Some(config_obj) =
                                source.get("config").and_then(|v| v.as_object())
                            {
                                if !config_obj.contains_key("endpoint") {
                                    return true;
                                }
                            } else {
                                // No config object or empty config = accepts POST
                                return true;
                            }
                        }
                    }
                }
            }

            // Check for to.format.output.mode === 'api' (exports via GET, cron disabled)
            if let Some(to) = step.get("to") {
                if let Some(to_type) = to.get("type").and_then(|v| v.as_str()) {
                    if to_type == "format" {
                        if let Some(output) = to.get("output") {
                            // Check if output is a string "api" or object with mode: "api"
                            if output.as_str() == Some("api") {
                                return true;
                            }
                            if let Some(output_obj) = output.as_object() {
                                if output_obj.get("mode").and_then(|v| v.as_str()) == Some("api") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Helper function to handle workflow-related errors
///
/// Maps `r_data_core_core::error::Error` variants to appropriate HTTP responses
pub(crate) fn handle_workflow_error(err: Error) -> HttpResponse {
    match err {
        Error::Database(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some("23505") =>
        {
            // Unique constraint violation
            ApiResponse::<()>::conflict("Workflow name already exists")
        }
        Error::Database(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some("23503") =>
        {
            // Foreign key constraint violation
            ApiResponse::<()>::bad_request("Invalid reference in workflow")
        }
        Error::Database(_) => {
            error!("Database error in workflow operation: {err}");
            ApiResponse::<()>::internal_error("Database error")
        }
        Error::Validation(msg) => ApiResponse::<()>::unprocessable_entity(&msg),
        Error::NotFound(msg) => ApiResponse::<()>::not_found(&msg),
        Error::Api(msg) => {
            error!("API error in workflow operation: {msg}");
            ApiResponse::<()>::internal_error("Workflow operation failed")
        }
        Error::Entity(msg) => {
            error!("Entity error in workflow operation: {msg}");
            ApiResponse::<()>::internal_error("Entity operation failed")
        }
        Error::Unknown(msg) => {
            error!("Unknown error in workflow operation: {msg}");
            ApiResponse::<()>::internal_error("Workflow operation failed")
        }
        _ => {
            error!("Unexpected workflow error: {err}");
            ApiResponse::<()>::internal_error("Internal server error")
        }
    }
}
