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
