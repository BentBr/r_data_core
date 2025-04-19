use actix_web::{error::ErrorUnauthorized, http::header, Error as ActixError, HttpRequest};
use log::{debug, error};
use sqlx::PgPool;
use std::result::Result as StdResult;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::jwt::{verify_jwt, AuthUserClaims};
use crate::entity::admin_user::ApiKey;
use crate::error::{Error, Result};

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
pub async fn extract_and_validate_api_key(
    req: &HttpRequest,
    pool: &PgPool,
) -> StdResult<Option<(ApiKey, Uuid)>, ActixError> {
    // Try API key header
    if let Some(api_key) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
        // Try to find the API key in the database
        match find_api_key(api_key, pool).await {
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
async fn find_api_key(
    api_key: &str,
    pool: &PgPool,
) -> StdResult<Option<(ApiKey, Uuid)>, sqlx::Error> {
    // Query the database for the API key
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            String,
            Option<String>,
            bool,
            OffsetDateTime,
            Option<OffsetDateTime>,
            Option<OffsetDateTime>,
            Option<Uuid>,
            bool,
        ),
    >(
        r#"
        SELECT 
            uuid,
            user_uuid,
            key_hash, 
            name, 
            description, 
            is_active, 
            created_at, 
            expires_at, 
            last_used_at,
            created_by,
            published
        FROM api_keys 
        WHERE key_hash = $1
        "#,
    )
    .bind(api_key)
    .fetch_optional(pool)
    .await?;

    // Check if the key exists
    if let Some((
        uuid,
        user_uuid,
        key_hash,
        name,
        description,
        is_active,
        created_at,
        expires_at,
        last_used_at,
        created_by,
        published,
    )) = row
    {
        // Ensure required fields are present
        if created_by.is_none() {
            error!("API key found but created_by is missing");
            return Ok(None);
        }

        // Convert row to ApiKey
        let key = ApiKey {
            uuid,
            user_uuid,
            key_hash,
            name,
            description,
            is_active,
            created_at,
            expires_at,
            last_used_at,
            created_by: created_by.unwrap(), // Safe to unwrap after check above
            published,
        };

        // Check if the key is active and not expired
        if key.is_active && is_key_valid(&key) {
            return Ok(Some((key, user_uuid)));
        }
    }

    Ok(None)
}

/// Update the last_used_at timestamp for an API key
async fn update_api_key_usage(key_uuid: Uuid, pool: &PgPool) -> StdResult<(), sqlx::Error> {
    let result = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE uuid = $2")
        .bind(OffsetDateTime::now_utc())
        .bind(key_uuid)
        .execute(pool)
        .await;

    if let Err(e) = result {
        error!("Failed to update last_used_at for API key: {}", e);
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

pub async fn authenticate_api_key(api_key: &str, pool: &PgPool) -> Result<Uuid> {
    if api_key.is_empty() {
        return Err(Error::Auth("No API key provided".to_string()));
    }

    // Try to find the API key directly in the database
    match find_api_key(api_key, pool).await {
        Ok(Some((_key, user_uuid))) => {
            // Authentication succeeded, return the user UUID
            Ok(user_uuid)
        }
        Ok(None) => {
            // API key not found, invalid, or expired
            Err(Error::Auth("Invalid API key".to_string()))
        }
        Err(e) => {
            error!("Database error during API key authentication: {}", e);
            Err(Error::Database(e))
        }
    }
}
