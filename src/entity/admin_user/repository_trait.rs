use crate::entity::admin_user::ApiKey;
use crate::error::Result;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository trait for API key operations
#[async_trait]
pub trait ApiKeyRepositoryTrait: Send + Sync {
    /// Find an API key by its value, optimized for authentication
    async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>>;

    /// Get a full API key by UUID
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>>;

    /// Create a new API key
    async fn create(&self, key: &ApiKey) -> Result<Uuid>;

    /// List all API keys for a user
    async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiKey>>;

    /// Revoke an API key (set is_active to false)
    async fn revoke(&self, uuid: Uuid) -> Result<()>;

    /// Get an API key by its name for a specific user
    async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>>;

    /// Get an API key by its hash value
    async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>>;

    /// Create a new API key with full details
    async fn create_new_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)>;

    /// Update an API key's last used timestamp
    async fn update_last_used(&self, uuid: Uuid) -> Result<()>;

    /// Reassign an API key to a different user
    async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()>;
}

/// Check if an API key is valid (not expired)
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
