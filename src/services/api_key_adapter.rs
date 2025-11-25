use r_data_core_persistence::{ApiKeyRepository, ApiKeyRepositoryTrait};
use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::error::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// Repository adapter for ApiKeyRepository
pub struct ApiKeyRepositoryAdapter {
    inner: ApiKeyRepository,
}

impl ApiKeyRepositoryAdapter {
    pub fn new(inner: ApiKeyRepository) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl ApiKeyRepositoryTrait for ApiKeyRepositoryAdapter {
    async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>> {
        self.inner.find_api_key_for_auth(api_key).await
    }

    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>> {
        self.inner.get_by_uuid(uuid).await
    }

    async fn create(&self, key: &ApiKey) -> Result<Uuid> {
        self.inner.create(key).await.map(|_| key.uuid)
    }

    async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiKey>> {
        self.inner.list_by_user(user_uuid, limit, offset).await
    }

    async fn revoke(&self, uuid: Uuid) -> Result<()> {
        self.inner.revoke(uuid).await
    }

    async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>> {
        self.inner.get_by_name(user_uuid, name).await
    }

    async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>> {
        self.inner.get_by_hash(api_key).await
    }

    async fn create_new_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        self.inner
            .create_new_api_key(name, description, created_by, expires_in_days)
            .await
    }

    async fn update_last_used(&self, uuid: Uuid) -> Result<()> {
        self.inner.update_last_used(uuid).await
    }

    async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()> {
        self.inner.reassign(uuid, new_user_uuid).await
    }

    async fn count_by_user(&self, user_uuid: Uuid) -> Result<i64> {
        self.inner.count_by_user(user_uuid).await
    }
}
