use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::entity::admin_user::UserRole;
use crate::entity::AdminUser;
use crate::error::{Error, Result};
use utoipa::ToSchema;

// Token expiry constants
pub const ACCESS_TOKEN_EXPIRY_SECONDS: u64 = 1800; // 30 minutes
pub const REFRESH_TOKEN_EXPIRY_SECONDS: u64 = 2592000; // 30 days

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

/// Generate an access JWT token for a user with short expiry
pub fn generate_access_token(user: &AdminUser, secret: &str) -> Result<String> {
    generate_jwt(user, secret, ACCESS_TOKEN_EXPIRY_SECONDS)
}

/// Generate a JWT token for a user
pub fn generate_jwt(user: &AdminUser, secret: &str, expiration_seconds: u64) -> Result<String> {
    let user_uuid = user.uuid;

    log::debug!("Generating JWT for user: {}", user.username);

    // Create expiration time
    let now = OffsetDateTime::now_utc();
    let expiration = now
        .checked_add(Duration::seconds(expiration_seconds as i64))
        .ok_or_else(|| Error::Auth("Could not create token expiration".to_string()))?;

    // Create claims
    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        name: user.username.clone(),
        email: user.email.clone(),
        is_admin: user.role == UserRole::Admin,
        role: format!("{:?}", user.role),
        exp: expiration.unix_timestamp() as usize,
        iat: now.unix_timestamp() as usize,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::admin_user::{AdminUser, UserRole, UserStatus};
    use crate::entity::AbstractRDataEntity;
    use std::collections::HashMap;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn create_test_user() -> AdminUser {
        let base = AbstractRDataEntity {
            uuid: Uuid::now_v7(),
            path: "/test/user".to_string(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            created_by: Uuid::now_v7(),
            updated_by: None,
            published: true,
            version: 1,
            custom_fields: HashMap::new(),
        };

        AdminUser {
            base,
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            full_name: "Test User".to_string(),
            role: UserRole::Admin,
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            permission_scheme_uuid: None,
            uuid: Uuid::now_v7(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            is_active: true,
            is_admin: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn test_generate_jwt_success() {
        let user = create_test_user();
        let secret = "test_secret_key";

        let result = generate_jwt(&user, secret, 3600);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_generate_access_token() {
        let user = create_test_user();
        let secret = "test_secret_key";

        let result = generate_access_token(&user, secret);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_jwt_success() {
        let user = create_test_user();
        let secret = "test_secret_key";

        let token = generate_jwt(&user, secret, 3600).unwrap();
        let result = verify_jwt(&token, secret);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, user.uuid.to_string());
        assert_eq!(claims.name, user.username);
        assert_eq!(claims.email, user.email);
        assert_eq!(claims.is_admin, true);
        assert_eq!(claims.role, "Admin");
    }

    #[test]
    fn test_verify_jwt_invalid_token() {
        let secret = "test_secret_key";
        let invalid_token = "invalid.jwt.token";

        let result = verify_jwt(invalid_token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_wrong_secret() {
        let user = create_test_user();
        let correct_secret = "correct_secret";
        let wrong_secret = "wrong_secret";

        let token = generate_jwt(&user, correct_secret, 3600).unwrap();
        let result = verify_jwt(&token, wrong_secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_expired_token() {
        let user = create_test_user();
        let secret = "test_secret_key";

        // Create a token that's already expired (expired 1 hour ago)
        let now = OffsetDateTime::now_utc();
        let expired_time = now - Duration::hours(1);

        let claims = AuthUserClaims {
            sub: user.uuid.to_string(),
            name: user.username.clone(),
            email: user.email.clone(),
            is_admin: true,
            role: "Admin".to_string(),
            exp: expired_time.unix_timestamp() as usize,
            iat: now.unix_timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let result = verify_jwt(&token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_user_claims_serialization() {
        let claims = AuthUserClaims {
            sub: "test-uuid".to_string(),
            name: "test_user".to_string(),
            email: "test@example.com".to_string(),
            is_admin: true,
            role: "Admin".to_string(),
            exp: OffsetDateTime::now_utc().unix_timestamp() as usize + 3600,
            iat: OffsetDateTime::now_utc().unix_timestamp() as usize,
        };

        let serialized = serde_json::to_string(&claims);
        assert!(serialized.is_ok());

        let deserialized: AuthUserClaims = serde_json::from_str(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.name, claims.name);
        assert_eq!(deserialized.email, claims.email);
        assert_eq!(deserialized.is_admin, claims.is_admin);
        assert_eq!(deserialized.role, claims.role);
    }

    #[test]
    fn test_generate_jwt_with_zero_expiry() {
        let user = create_test_user();
        let secret = "test_secret_key";

        // This should fail because we can't add 0 seconds to now
        let result = generate_jwt(&user, secret, 0);
        assert!(result.is_ok()); // Actually this might work, let's see
    }

    #[test]
    fn test_generate_jwt_with_very_long_expiry() {
        let user = create_test_user();
        let secret = "test_secret_key";

        // Test with a very long expiry (100 years)
        let result = generate_jwt(&user, secret, 3153600000);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }
}
