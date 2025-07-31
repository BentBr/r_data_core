use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Refresh token for secure authentication
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub is_revoked: bool,
    pub device_info: Option<serde_json::Value>,
}

/// Request to create a new refresh token
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRefreshTokenRequest {
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub device_info: Option<serde_json::Value>,
}

/// Response after creating a refresh token
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
    pub token: String, // The actual token (not stored in DB)
    pub expires_at: OffsetDateTime,
}

impl RefreshToken {
    /// Create a new refresh token instance
    pub fn new(
        user_id: Uuid,
        token_hash: String,
        expires_at: OffsetDateTime,
        device_info: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
            user_id,
            token_hash,
            expires_at,
            created_at: OffsetDateTime::now_utc(),
            last_used_at: None,
            is_revoked: false,
            device_info,
        }
    }

    /// Check if the refresh token is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at <= OffsetDateTime::now_utc()
    }

    /// Check if the refresh token is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked
    }

    /// Mark the token as used
    pub fn mark_as_used(&mut self) {
        self.last_used_at = Some(OffsetDateTime::now_utc());
    }

    /// Revoke the token
    pub fn revoke(&mut self) {
        self.is_revoked = true;
    }

    /// Generate a secure random token
    pub fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::rng();
        let token_bytes: [u8; 32] = rng.random();
        base64::encode(token_bytes)
    }

    /// Hash a token for storage
    pub fn hash_token(token: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

// RefreshToken doesn't need to implement AbstractRDataEntity since it has its own fields

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_refresh_token_new() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let token_hash = "test_hash_123".to_string();
        let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
        let device_info = serde_json::json!({"device": "test"});

        let token = RefreshToken::new(
            user_id,
            token_hash.clone(),
            expires_at,
            Some(device_info.clone()),
        );

        assert_eq!(token.user_id, user_id);
        assert_eq!(token.token_hash, token_hash);
        assert_eq!(token.expires_at, expires_at);
        assert_eq!(token.device_info, Some(device_info));
        assert!(!token.is_revoked);
        assert!(token.last_used_at.is_none());
    }

    #[test]
    fn test_refresh_token_is_expired() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let token_hash = "test_hash".to_string();

        // Create expired token
        let expired_at = OffsetDateTime::now_utc() - Duration::hours(1);
        let expired_token = RefreshToken::new(user_id, token_hash.clone(), expired_at, None);
        assert!(expired_token.is_expired());

        // Create valid token
        let valid_until = OffsetDateTime::now_utc() + Duration::hours(1);
        let valid_token = RefreshToken::new(user_id, token_hash, valid_until, None);
        assert!(!valid_token.is_expired());
    }

    #[test]
    fn test_refresh_token_is_valid() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let token_hash = "test_hash".to_string();

        // Create valid token
        let valid_until = OffsetDateTime::now_utc() + Duration::hours(1);
        let mut valid_token = RefreshToken::new(user_id, token_hash.clone(), valid_until, None);
        assert!(valid_token.is_valid());

        // Revoke the token
        valid_token.revoke();
        assert!(!valid_token.is_valid());

        // Create expired token
        let expired_at = OffsetDateTime::now_utc() - Duration::hours(1);
        let expired_token = RefreshToken::new(user_id, token_hash, expired_at, None);
        assert!(!expired_token.is_valid());
    }

    #[test]
    fn test_refresh_token_mark_as_used() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let token_hash = "test_hash".to_string();
        let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

        let mut token = RefreshToken::new(user_id, token_hash, expires_at, None);
        assert!(token.last_used_at.is_none());

        let before_mark = OffsetDateTime::now_utc();
        token.mark_as_used();
        let after_mark = OffsetDateTime::now_utc();

        assert!(token.last_used_at.is_some());
        let last_used = token.last_used_at.unwrap();
        assert!(last_used >= before_mark && last_used <= after_mark);
    }

    #[test]
    fn test_refresh_token_revoke() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let token_hash = "test_hash".to_string();
        let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

        let mut token = RefreshToken::new(user_id, token_hash, expires_at, None);
        assert!(!token.is_revoked);

        token.revoke();
        assert!(token.is_revoked);
    }

    #[test]
    fn test_refresh_token_generate_token() {
        let token1 = RefreshToken::generate_token();
        let token2 = RefreshToken::generate_token();

        // Tokens should be different
        assert_ne!(token1, token2);

        // Tokens should be base64 encoded strings
        assert!(!token1.is_empty());
        assert!(!token2.is_empty());

        // Tokens should be reasonably long (32 bytes = 44 base64 chars)
        assert!(token1.len() >= 40);
        assert!(token2.len() >= 40);
    }

    #[test]
    fn test_refresh_token_hash_token() {
        let token = "test_token_123";
        let hash1 = RefreshToken::hash_token(token).unwrap();
        let hash2 = RefreshToken::hash_token(token).unwrap();

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be different from original token
        assert_ne!(hash1, token);

        // Hash should be a hex string
        assert!(hash1.len() == 64); // SHA256 produces 32 bytes = 64 hex chars

        // Test with different token
        let different_token = "different_token_456";
        let different_hash = RefreshToken::hash_token(different_token).unwrap();
        assert_ne!(hash1, different_hash);
    }

    #[test]
    fn test_refresh_token_hash_token_empty() {
        let hash = RefreshToken::hash_token("").unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 of empty string
    }

    #[test]
    fn test_create_refresh_token_request() {
        let user_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let expires_at = OffsetDateTime::now_utc() + Duration::days(30);
        let device_info = serde_json::json!({"device": "test"});

        let request = CreateRefreshTokenRequest {
            user_id,
            expires_at,
            device_info: Some(device_info.clone()),
        };

        assert_eq!(request.user_id, user_id);
        assert_eq!(request.expires_at, expires_at);
        assert_eq!(request.device_info, Some(device_info));
    }

    #[test]
    fn test_refresh_token_response() {
        let token = "test_token_string";
        let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

        let response = RefreshTokenResponse {
            token: token.to_string(),
            expires_at,
        };

        assert_eq!(response.token, token);
        assert_eq!(response.expires_at, expires_at);
    }
}
