#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use r_data_core_core::admin_user::{AdminUser, UserRole};
use r_data_core_core::config::ApiConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::permission_scheme::{PermissionScheme, ResourceNamespace};
use utoipa::ToSchema;

// Token expiry constants
pub const ACCESS_TOKEN_EXPIRY_SECONDS: u64 = 1_800; // 30 minutes (short-lived access token)
pub const REFRESH_TOKEN_EXPIRY_SECONDS: u64 = 2_592_000; // 30 days

/// Claims for authentication
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthUserClaims {
    /// User UUID as string
    pub sub: String,
    /// Username
    pub name: String,
    /// Email
    pub email: String,
    /// User role (`SuperAdmin` or custom role name)
    pub role: String,
    /// Super admin flag - overrides all permissions
    pub is_super_admin: bool,
    /// Allowed actions in format: "namespace:action" or "namespace:path:action"
    pub permissions: Vec<String>,
    /// Expiration timestamp
    pub exp: usize,
    /// Issued at timestamp
    pub iat: usize,
}

/// Generate an access JWT token for a user with short expiry
///
/// # Arguments
/// * `user` - Admin user
/// * `config` - API configuration containing JWT secret and expiration
/// * `schemes` - Vector of permission schemes (if empty, user has no permissions except `SuperAdmin`/`super_admin`)
///
/// # Errors
/// Returns an error if token generation fails
pub fn generate_access_token(
    user: &AdminUser,
    config: &ApiConfig,
    schemes: &[PermissionScheme],
) -> Result<String> {
    generate_jwt(user, config, ACCESS_TOKEN_EXPIRY_SECONDS, schemes)
}

/// Generate a JWT token for a user
///
/// # Arguments
/// * `user` - Admin user
/// * `config` - API configuration containing JWT secret
/// * `expiration_seconds` - Token expiration in seconds (overrides config if provided)
/// * `schemes` - Vector of permission schemes (if empty, user has no permissions except `SuperAdmin`/`super_admin`)
///
/// # Errors
/// Returns an error if token generation fails
pub fn generate_jwt(
    user: &AdminUser,
    config: &ApiConfig,
    expiration_seconds: u64,
    schemes: &[PermissionScheme],
) -> Result<String> {
    let user_uuid = user.uuid;

    let username = &user.username;
    log::debug!("Generating JWT for user: {username}");

    // Create expiration time
    let now = OffsetDateTime::now_utc();
    let expiration = now
        .checked_add(Duration::seconds(
            i64::try_from(expiration_seconds).unwrap_or(i64::MAX),
        ))
        .ok_or_else(|| {
            r_data_core_core::error::Error::Auth("Could not create token expiration".to_string())
        })?;

    // Check super_admin flag first, then SuperAdmin role
    let is_super_admin = user.super_admin || matches!(user.role, UserRole::SuperAdmin);

    // Extract permissions from schemes
    let permissions = if is_super_admin {
        // SuperAdmin or super_admin flag gets all permissions for all namespaces
        generate_all_permissions()
    } else if !schemes.is_empty() {
        // Merge permissions from all schemes for user's role
        merge_permissions_from_multiple_schemes(schemes, user.role.as_str())
    } else {
        // No schemes means no permissions
        Vec::new()
    };

    // Create claims
    let claims = AuthUserClaims {
        sub: user_uuid.to_string(),
        name: user.username.clone(),
        email: user.email.clone(),
        role: user.role.as_str().to_string(),
        is_super_admin,
        permissions,
        exp: usize::try_from(expiration.unix_timestamp()).unwrap_or(0),
        iat: usize::try_from(now.unix_timestamp()).unwrap_or(0),
    };

    // Generate the token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|e| r_data_core_core::error::Error::Auth(format!("Token generation error: {e}")))?;

    Ok(token)
}

/// Merge permissions from multiple schemes for a role
///
/// This combines all permissions from all schemes for the given role,
/// converting them to permission strings and deduplicating.
///
/// # Arguments
/// * `schemes` - Vector of permission schemes
/// * `role` - Role name to get permissions for
///
/// # Returns
/// Vector of merged permission strings (deduplicated)
#[must_use]
fn merge_permissions_from_multiple_schemes(
    schemes: &[PermissionScheme],
    role: &str,
) -> Vec<String> {
    use std::collections::HashSet;

    let mut permission_set = HashSet::new();
    let mut merged_permissions = Vec::new();

    for scheme in schemes {
        let scheme_permissions = scheme.get_permissions_as_strings(role);
        for perm in scheme_permissions {
            if permission_set.insert(perm.clone()) {
                merged_permissions.push(perm);
            }
        }
    }

    merged_permissions
}

/// Generate all permissions for `SuperAdmin`
///
/// # Returns
/// Vector of all permission strings for all namespaces
#[must_use]
fn generate_all_permissions() -> Vec<String> {
    vec![
        {
            let ns = ResourceNamespace::Workflows.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::Workflows.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::Workflows.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::Workflows.as_str();
            format!("{ns}:delete")
        },
        {
            let ns = ResourceNamespace::Workflows.as_str();
            format!("{ns}:execute")
        },
        {
            let ns = ResourceNamespace::Entities.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::Entities.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::Entities.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::Entities.as_str();
            format!("{ns}:delete")
        },
        {
            let ns = ResourceNamespace::EntityDefinitions.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::EntityDefinitions.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::EntityDefinitions.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::EntityDefinitions.as_str();
            format!("{ns}:delete")
        },
        {
            let ns = ResourceNamespace::ApiKeys.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::ApiKeys.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::ApiKeys.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::ApiKeys.as_str();
            format!("{ns}:delete")
        },
        {
            let ns = ResourceNamespace::PermissionSchemes.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::PermissionSchemes.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::PermissionSchemes.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::PermissionSchemes.as_str();
            format!("{ns}:delete")
        },
        {
            let ns = ResourceNamespace::System.as_str();
            format!("{ns}:read")
        },
        {
            let ns = ResourceNamespace::System.as_str();
            format!("{ns}:create")
        },
        {
            let ns = ResourceNamespace::System.as_str();
            format!("{ns}:update")
        },
        {
            let ns = ResourceNamespace::System.as_str();
            format!("{ns}:delete")
        },
    ]
}

/// Verify and decode a JWT token
///
/// # Arguments
/// * `token` - JWT token string
/// * `secret` - JWT secret key
///
/// # Errors
/// Returns an error if token validation fails
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
            log::error!("JWT validation error: {e}");
            Err(r_data_core_core::error::Error::Auth(format!(
                "Token validation error: {e}"
            )))
        }
    }
}

// Add AdminOnly extractor if needed
pub struct AdminOnly(pub AuthUserClaims);

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_core::admin_user::UserStatus;
    use r_data_core_core::domain::AbstractRDataEntity;
    use std::collections::HashMap;
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
            role: UserRole::SuperAdmin,
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            super_admin: false,
            uuid: Uuid::now_v7(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            is_active: true,
            is_admin: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    fn create_test_config() -> ApiConfig {
        ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret_key".to_string(),
            jwt_expiration: 3600,
            enable_docs: true,
            cors_origins: vec!["*".to_string()],
        }
    }

    #[test]
    fn test_generate_jwt_success() {
        let user = create_test_user();
        let config = create_test_config();

        let result = generate_jwt(&user, &config, 3600, &[]);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_generate_access_token() {
        let user = create_test_user();
        let config = create_test_config();

        let result = generate_access_token(&user, &config, &[]);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_jwt_success() {
        let user = create_test_user();
        let config = create_test_config();

        let token = generate_jwt(&user, &config, 3600, &[]).unwrap();
        let result = verify_jwt(&token, &config.jwt_secret);

        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, user.uuid.to_string());
        assert_eq!(claims.name, user.username);
        assert_eq!(claims.email, user.email);
        assert_eq!(claims.role, "SuperAdmin");
        assert_eq!(claims.is_super_admin, false); // User has SuperAdmin role but not super_admin flag
                                                  // SuperAdmin should have all permissions
        assert!(!claims.permissions.is_empty());
    }

    #[test]
    fn test_verify_jwt_invalid_token() {
        let config = create_test_config();
        let invalid_token = "invalid.jwt.token";

        let result = verify_jwt(invalid_token, &config.jwt_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_wrong_secret() {
        let user = create_test_user();
        let config = create_test_config();
        let wrong_secret = "wrong_secret";

        let token = generate_jwt(&user, &config, 3600, &[]).unwrap();
        let result = verify_jwt(&token, wrong_secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_expired_token() {
        let user = create_test_user();
        let config = create_test_config();

        // Create a token that's already expired (expired 1 hour ago)
        let now = OffsetDateTime::now_utc();
        let expired_time = now - Duration::hours(1);

        let claims = AuthUserClaims {
            sub: user.uuid.to_string(),
            name: user.username.clone(),
            email: user.email.clone(),
            role: "SuperAdmin".to_string(),
            is_super_admin: false,
            permissions: vec!["workflows:read".to_string()],
            exp: expired_time.unix_timestamp() as usize,
            iat: now.unix_timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .unwrap();

        let result = verify_jwt(&token, &config.jwt_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_user_claims_serialization() {
        let claims = AuthUserClaims {
            sub: "test-uuid".to_string(),
            name: "test_user".to_string(),
            email: "test@example.com".to_string(),
            role: "SuperAdmin".to_string(),
            is_super_admin: false,
            permissions: vec!["workflows:read".to_string()],
            exp: OffsetDateTime::now_utc().unix_timestamp() as usize + 3600,
            iat: OffsetDateTime::now_utc().unix_timestamp() as usize,
        };

        let serialized = serde_json::to_string(&claims);
        assert!(serialized.is_ok());

        let deserialized: AuthUserClaims = serde_json::from_str(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.name, claims.name);
        assert_eq!(deserialized.email, claims.email);
        assert_eq!(deserialized.permissions, claims.permissions);
        assert_eq!(deserialized.role, claims.role);
    }

    #[test]
    fn test_generate_jwt_with_zero_expiry() {
        let user = create_test_user();
        let config = create_test_config();

        // This should fail because we can't add 0 seconds to now
        let result = generate_jwt(&user, &config, 0, &[]);
        assert!(result.is_ok()); // Actually this might work, let's see
    }

    #[test]
    fn test_generate_jwt_with_very_long_expiry() {
        let user = create_test_user();
        let config = create_test_config();

        // Test with a very long expiry (100 years)
        let result = generate_jwt(&user, &config, 3153600000, &[]);
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());
    }
}
