use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::admin_user::UserRole;
use crate::entity::AdminUser;
use crate::error::{Error, Result};
use utoipa::ToSchema;

/// Claims for authentication
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthUserClaims {
    pub sub: String,    // User UUID as string
    pub name: String,   // Username
    pub email: String,  // Email
    pub is_admin: bool, // Admin flag
    pub role: String,   // User role
    pub exp: usize,     // Expiration timestamp
    pub iat: usize,     // Issued at timestamp
}

/// Generate a JWT token for a user
pub fn generate_jwt(user: &AdminUser, secret: &str, expiration_seconds: u64) -> Result<String> {
    let user_uuid = user.uuid;

    log::debug!("Generating JWT for user: {}", user.username);

    // Create expiration time
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(expiration_seconds as i64))
        .ok_or_else(|| Error::Auth("Could not create token expiration".to_string()))?;

    // Create claims
    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        name: user.username.clone(),
        email: user.email.clone(),
        is_admin: user.role == UserRole::Admin,
        role: format!("{:?}", user.role),
        exp: expiration.timestamp() as usize,
        iat: Utc::now().timestamp() as usize,
    };

    // Generate the token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| Error::Auth(format!("Token generation error: {}", e)))?;

    Ok(token)
}

/// Verify and decode a JWT token
pub fn verify_jwt(token: &str, secret: &str) -> Result<AuthUserClaims> {
    // Decode and validate the token with minimal logging
    let validation = Validation::default();

    match decode::<AuthUserClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    ) {
        Ok(token_data) => Ok(token_data.claims),
        Err(e) => {
            log::error!("JWT validation error: {}", e);
            Err(Error::Auth(format!("Token validation error: {}", e)))
        }
    }
}

// Add AdminOnly extractor if needed
pub struct AdminOnly(pub AuthUserClaims);
