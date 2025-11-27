use actix_web::{
    error::{Error as ActixError, ErrorUnauthorized},
    http::header,
    web, HttpRequest,
};
use log::{debug, error};
use uuid::Uuid;

use r_data_core_api::jwt::{verify_jwt, AuthUserClaims};
use r_data_core_api::api_state::ApiStateTrait;
use crate::entity::admin_user::ApiKey;

use std::result::Result as StdResult;

/// Extract and validate JWT token from request headers
pub async fn extract_and_validate_jwt(
    req: &HttpRequest,
    jwt_secret: &str,
) -> StdResult<Option<AuthUserClaims>, ActixError> {
    // Extract Authorization header
    if let Some(auth_header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        // Check for Bearer prefix
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..]; // Remove "Bearer " prefix

            // Only log token length, not the token itself
            debug!(
                "Processing JWT token for path: {} (token length: {})",
                req.path(),
                token.len()
            );

            // Verify JWT token
            return match verify_jwt(token, jwt_secret) {
                Ok(claims) => {
                    debug!("JWT auth successful for user: {}", claims.name);
                    Ok(Some(claims))
                }
                Err(err) => {
                    log::error!("JWT verification failed: {}", err);
                    Ok(None)
                }
            };
        }
    }

    debug!("No valid JWT token found for path: {}", req.path());
    Ok(None)
}

/// Extract and validate API key from request headers
/// This function uses ApiKeyService with caching support
/// ApiState should always be available in normal operation
pub async fn extract_and_validate_api_key(
    req: &HttpRequest,
) -> StdResult<Option<(ApiKey, Uuid)>, ActixError> {
    // Try API key header
    if let Some(api_key) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
        // Get ApiState from request - should always be available
        // Try ApiStateWrapper first (for migrated routes), then fall back to ApiState (for old routes)
        if let Some(state) = req.app_data::<web::Data<r_data_core_api::ApiStateWrapper>>() {
            // Use ApiKeyService which handles caching (cache hit) or DB query (cache miss)
            // ApiStateTrait provides the api_key_service() method
            match state.api_key_service().validate_api_key(api_key).await {
                Ok(Some((key, user_uuid))) => {
                    debug!("API key authentication successful");
                    return Ok(Some((key, user_uuid)));
                }
                Ok(None) => {
                    debug!("API key not found or inactive");
                }
                Err(e) => {
                    error!("API key validation error: {}", e);
                    return Err(ErrorUnauthorized(
                        "Internal server error during API key validation",
                    ));
                }
            }
        } else {
            // This should not happen in normal operation, but provide fallback for safety
            error!("ApiStateWrapper not available in app_data - this indicates a configuration issue");
            return Err(ErrorUnauthorized(
                "API authentication not properly configured",
            ));
        }
    }

    Ok(None)
}
