#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Functions take &HttpRequest which is !Send

use actix_web::{
    error::{Error as ActixError, ErrorUnauthorized},
    http::header,
    web, HttpRequest,
};
use log::{debug, error};
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use r_data_core_core::admin_jwt::{verify_jwt, AuthUserClaims};
use r_data_core_core::admin_user::ApiKey;

use std::result::Result as StdResult;

/// Extract JWT token string from Authorization header (removes "Bearer " prefix)
/// Returns None if no valid Authorization header is found
#[must_use]
pub fn extract_jwt_token_string(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|auth_header| auth_header.strip_prefix("Bearer "))
}

/// Extract and validate JWT token from request headers
///
/// # Errors
/// Returns an error if JWT validation fails
pub async fn extract_and_validate_jwt(
    req: &HttpRequest,
    jwt_secret: &str,
) -> StdResult<Option<AuthUserClaims>, ActixError> {
    // Extract JWT token string
    if let Some(token) = extract_jwt_token_string(req) {
        // Only log token length, not the token itself
        debug!(
            "Processing JWT token for path: {} (token length: {})",
            req.path(),
            token.len()
        );

        // Verify JWT token
        return match verify_jwt(token, jwt_secret) {
            Ok(claims) => {
                let name = &claims.name;
                debug!("JWT auth successful for user: {name}");
                Ok(Some(claims))
            }
            Err(err) => {
                log::error!("JWT verification failed: {err}");
                Ok(None)
            }
        };
    }

    let path = req.path();
    debug!("No valid JWT token found for path: {path}");
    Ok(None)
}

/// Extract and validate API key from request headers
/// This function uses `ApiKeyService` with caching support
/// `ApiState` should always be available in normal operation
///
/// # Errors
/// Returns an error if API key validation fails or if the API state is missing
pub async fn extract_and_validate_api_key(
    req: &HttpRequest,
) -> StdResult<Option<(ApiKey, Uuid)>, ActixError> {
    // Try API key header
    if let Some(api_key) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
        // Trim any whitespace that might have been added
        let api_key = api_key.trim();
        debug!(
            "Found X-API-Key header, attempting validation (length: {})",
            api_key.len()
        );
        // Get ApiState from request - should always be available
        if let Some(state) = req.app_data::<web::Data<ApiStateWrapper>>() {
            debug!("Found ApiStateWrapper in app_data");
            // Use ApiKeyService which handles caching (cache hit) or DB query (cache miss)
            let validation_result = state.api_key_service().validate_api_key(api_key).await;
            match validation_result {
                Ok(Some((key, user_uuid))) => {
                    debug!("API key authentication successful for user: {user_uuid}");
                    return Ok(Some((key, user_uuid)));
                }
                Ok(None) => {
                    debug!(
                        "API key not found or inactive: key length = {}",
                        api_key.len()
                    );
                }
                Err(e) => {
                    error!("API key validation error: {e}");
                    return Err(ErrorUnauthorized(
                        "Internal server error during API key validation",
                    ));
                }
            }
        } else {
            // This should not happen in normal operation, but provide fallback for safety
            error!(
                "ApiStateWrapper not available in app_data - this indicates a configuration issue"
            );
            return Err(ErrorUnauthorized(
                "API authentication not properly configured",
            ));
        }
    } else {
        debug!("No X-API-Key header found");
    }

    Ok(None)
}
