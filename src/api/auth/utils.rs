use actix_web::{error::ErrorUnauthorized, http::header, web, HttpRequest};
use log::{debug, error};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::jwt::{verify_jwt, AuthUserClaims};
use crate::entity::admin_user::ApiKey;

/// Extract and validate JWT token from request headers
pub async fn extract_and_validate_jwt(
    req: &HttpRequest,
    jwt_secret: &str,
) -> Result<Option<AuthUserClaims>, actix_web::Error> {
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
pub async fn extract_and_validate_api_key(
    req: &HttpRequest,
    pool: &PgPool,
) -> Result<Option<(ApiKey, Uuid)>, actix_web::Error> {
    // Try API key header
    if let Some(api_key) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
        // Simplified approach to avoid SQL errors in development
        // First, try to find the API key in the database
        let maybe_key = find_api_key(api_key, pool).await;

        match maybe_key {
            Ok(Some((key, user_uuid))) => {
                // Then, try to update the last_used_at timestamp
                let _ = update_api_key_usage(key.uuid, pool).await;

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
    }

    Ok(None)
}

/// Find an API key in the database
async fn find_api_key(api_key: &str, pool: &PgPool) -> Result<Option<(ApiKey, Uuid)>, sqlx::Error> {
    // Safely check if the table exists first
    let table_exists = sqlx::query!(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'api_keys')"
    )
    .fetch_one(pool)
    .await?
    .exists
    .unwrap_or(false);

    if !table_exists {
        debug!("api_keys table does not exist");
        return Ok(None);
    }

    // Query the database for the API key
    let maybe_key = sqlx::query_as!(
        ApiKey,
        r#"
        SELECT 
            uuid,
            user_uuid,
            key_hash as "api_key", 
            name, 
            description, 
            is_active, 
            created_at, 
            expires_at, 
            last_used_at 
        FROM api_keys 
        WHERE key_hash = $1
        "#,
        api_key
    )
    .fetch_optional(pool)
    .await?;

    // Check if the key exists
    if let Some(key) = maybe_key {
        // Check if the key is active and not expired
        if key.is_active && is_key_valid(&key) {
            // Extract user_uuid first before key is moved
            let user_uuid = key.user_uuid;
            return Ok(Some((key, user_uuid)));
        }
    }

    Ok(None)
}

/// Update the last_used_at timestamp for an API key
async fn update_api_key_usage(key_uuid: Option<Uuid>, pool: &PgPool) -> Result<(), sqlx::Error> {
    if let Some(uuid) = key_uuid {
        let result = sqlx::query!(
            "UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2",
            OffsetDateTime::now_utc(),
            uuid
        )
        .execute(pool)
        .await;

        if let Err(e) = result {
            error!("Failed to update last_used_at for API key: {}", e);
        }
    }

    Ok(())
}

/// Check if an API key is still valid (not expired)
pub fn is_key_valid(key: &ApiKey) -> bool {
    if !key.is_active {
        return false;
    }

    if let Some(expires_at) = key.expires_at {
        if expires_at < OffsetDateTime::now_utc() {
            return false;
        }
    }

    true
}
