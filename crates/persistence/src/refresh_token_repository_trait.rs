use async_trait::async_trait;
use r_data_core_core::error::Result;
use r_data_core_core::refresh_token::RefreshToken;
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait RefreshTokenRepositoryTrait: Send + Sync {
    /// Create a new refresh token
    async fn create(
        &self,
        user_id: Uuid,
        token_hash: String,
        expires_at: OffsetDateTime,
        device_info: Option<serde_json::Value>,
    ) -> Result<RefreshToken>;

    /// Find a refresh token by its hash
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>>;

    /// Update last used timestamp for a refresh token
    async fn update_last_used(&self, id: Uuid) -> Result<()>;

    /// Revoke a refresh token by ID
    async fn revoke_by_id(&self, id: Uuid) -> Result<()>;

    /// Revoke a refresh token by token hash
    async fn revoke_by_token_hash(&self, token_hash: &str) -> Result<()>;

    /// Revoke all refresh tokens for a user
    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64>;

    /// Get all active refresh tokens for a user
    async fn get_active_tokens_for_user(&self, user_id: Uuid) -> Result<Vec<RefreshToken>>;

    /// Clean up expired and revoked tokens
    async fn cleanup_expired_tokens(&self) -> Result<u64>;

    /// Get refresh token statistics for a user
    async fn get_user_token_count(&self, user_id: Uuid) -> Result<i64>;

    /// Delete a refresh token by ID
    async fn delete_by_id(&self, id: Uuid) -> Result<()>;
}
