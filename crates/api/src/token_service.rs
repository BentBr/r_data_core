#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::config::ApiConfig;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::Role;
use r_data_core_core::refresh_token::RefreshToken;
use time::{Duration, OffsetDateTime};

use crate::jwt::{
    generate_access_token, ACCESS_TOKEN_EXPIRY_SECONDS, REFRESH_TOKEN_EXPIRY_SECONDS,
};

/// All token data produced by a single token-pair generation
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_hash: String,
    pub access_expires_at: OffsetDateTime,
    pub refresh_expires_at: OffsetDateTime,
}

/// Service responsible for generating auth token pairs.
///
/// Encapsulates access/refresh token generation so route handlers stay thin.
pub struct TokenService<'a> {
    api_config: &'a ApiConfig,
}

impl<'a> TokenService<'a> {
    #[must_use]
    pub const fn new(api_config: &'a ApiConfig) -> Self {
        Self { api_config }
    }

    /// Generate an access + refresh token pair for the given user and roles.
    ///
    /// # Errors
    /// Returns an error if JWT signing or refresh-token hashing fails.
    pub fn generate_token_pair(&self, user: &AdminUser, roles: &[Role]) -> Result<TokenPair> {
        let access_token = generate_access_token(user, self.api_config, roles)?;

        let refresh_token = RefreshToken::generate_token();
        let refresh_token_hash = RefreshToken::hash_token(&refresh_token)?;

        let access_expires_at = OffsetDateTime::now_utc()
            .checked_add(Duration::seconds(
                i64::try_from(ACCESS_TOKEN_EXPIRY_SECONDS).unwrap_or(0),
            ))
            .unwrap_or_else(OffsetDateTime::now_utc);

        let refresh_expires_at = OffsetDateTime::now_utc()
            .checked_add(Duration::seconds(
                i64::try_from(REFRESH_TOKEN_EXPIRY_SECONDS).unwrap_or(0),
            ))
            .unwrap_or_else(OffsetDateTime::now_utc);

        Ok(TokenPair {
            access_token,
            refresh_token,
            refresh_token_hash,
            access_expires_at,
            refresh_expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_core::admin_user::UserStatus;
    use r_data_core_core::domain::AbstractRDataEntity;
    use uuid::Uuid;

    fn test_config() -> ApiConfig {
        ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 8888,
            use_tls: false,
            jwt_secret: "test_secret_key_for_token_service".to_string(),
            jwt_expiration: 3600,
            enable_docs: false,
            cors_origins: vec![],
            check_default_admin_password: false,
        }
    }

    fn test_user() -> AdminUser {
        let now = OffsetDateTime::now_utc();
        AdminUser {
            uuid: Uuid::now_v7(),
            username: "tokentest".to_string(),
            email: "token@test.com".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Token Test".to_string(),
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            super_admin: true,
            first_name: Some("Token".to_string()),
            last_name: Some("Test".to_string()),
            is_active: true,
            is_admin: true,
            created_at: now,
            updated_at: now,
            base: AbstractRDataEntity::new("/test".to_string()),
        }
    }

    #[test]
    fn generate_token_pair_returns_non_empty_tokens() {
        let config = test_config();
        let svc = TokenService::new(&config);
        let pair = svc.generate_token_pair(&test_user(), &[]).unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert!(!pair.refresh_token_hash.is_empty());
        assert!(pair.access_expires_at > OffsetDateTime::now_utc());
        assert!(pair.refresh_expires_at > pair.access_expires_at);
    }

    #[test]
    fn refresh_token_hash_is_deterministic_for_same_input() {
        let token = RefreshToken::generate_token();
        let h1 = RefreshToken::hash_token(&token).unwrap();
        let h2 = RefreshToken::hash_token(&token).unwrap();
        assert_eq!(h1, h2);
    }
}
