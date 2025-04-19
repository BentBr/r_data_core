use crate::{
    entity::admin_user::{ApiKey, ApiKeyRepository, ApiKeyRepositoryTrait},
    error::{Error, Result},
};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Service for handling API key operations
pub struct ApiKeyService {
    repository: Arc<dyn ApiKeyRepositoryTrait>,
}

impl ApiKeyService {
    /// Create a new API key service with a concrete repository
    pub fn new(repository: Arc<dyn ApiKeyRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Create a new API key service from a concrete repository
    pub fn from_repository(repository: ApiKeyRepository) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }

    /// Create a new API key
    pub async fn create_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        // Validation
        if name.is_empty() {
            return Err(Error::Validation(
                "API key name cannot be empty".to_string(),
            ));
        }

        if expires_in_days < 0 {
            return Err(Error::Validation(
                "Expiration days cannot be negative".to_string(),
            ));
        }

        self.repository
            .create_new_api_key(name, description, created_by, expires_in_days)
            .await
    }

    /// Validate an API key and return user information if valid
    pub async fn validate_api_key(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>> {
        if api_key.is_empty() {
            return Ok(None);
        }

        self.repository.find_api_key_for_auth(api_key).await
    }

    /// List all API keys for a user
    pub async fn list_keys_for_user(
        &self,
        user_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>> {
        // Validate input
        let limit = if limit <= 0 { 50 } else { limit };
        let offset = if offset < 0 { 0 } else { offset };

        self.repository.list_by_user(user_uuid, limit, offset).await
    }

    /// Revoke an API key
    pub async fn revoke_key(&self, key_uuid: Uuid, user_uuid: Uuid) -> Result<()> {
        // Verify ownership first
        let key = self.repository.get_by_uuid(key_uuid).await?;

        match key {
            Some(key) if key.user_uuid == user_uuid => self.repository.revoke(key_uuid).await,
            Some(_) => Err(Error::Forbidden(
                "You don't have permission to revoke this API key".to_string(),
            )),
            None => Err(Error::NotFound("API key not found".to_string())),
        }
    }

    /// Get a key by UUID
    pub async fn get_key(&self, key_uuid: Uuid) -> Result<Option<ApiKey>> {
        self.repository.get_by_uuid(key_uuid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use time::OffsetDateTime;

    mock! {
        pub ApiKeyRepo {}

        #[async_trait]
        impl ApiKeyRepositoryTrait for ApiKeyRepo {
            async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>>;
            async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>>;
            async fn create(&self, key: &ApiKey) -> Result<Uuid>;
            async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64) -> Result<Vec<ApiKey>>;
            async fn revoke(&self, uuid: Uuid) -> Result<()>;
            async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>>;
            async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>>;
            async fn create_new_api_key(
                &self,
                name: &str,
                description: &str,
                created_by: Uuid,
                expires_in_days: i32,
            ) -> Result<(Uuid, String)>;
            async fn update_last_used(&self, uuid: Uuid) -> Result<()>;
            async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_create_api_key_with_valid_input() {
        let mut mock_repo = MockApiKeyRepo::new();

        let user_uuid = Uuid::now_v7();
        let key_uuid = Uuid::now_v7();
        let key_value = "test_api_key_12345".to_string();

        mock_repo
            .expect_create_new_api_key()
            .with(
                eq("Test Key"),
                eq("Test Description"),
                eq(user_uuid),
                eq(30),
            )
            .returning(move |_, _, _, _| Ok((key_uuid, key_value.to_string())));

        let service = ApiKeyService::new(Arc::new(mock_repo));
        let result = service
            .create_api_key("Test Key", "Test Description", user_uuid, 30)
            .await;

        assert!(result.is_ok());
        let (uuid, key) = result.unwrap();
        assert_eq!(uuid, key_uuid);
        assert_eq!(key, "test_api_key_12345");
    }

    #[tokio::test]
    async fn test_create_api_key_with_empty_name() {
        let mock_repo = MockApiKeyRepo::new();
        let user_uuid = Uuid::now_v7();

        let service = ApiKeyService::new(Arc::new(mock_repo));
        let result = service
            .create_api_key("", "Test Description", user_uuid, 30)
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Validation(msg)) => {
                assert_eq!(msg, "API key name cannot be empty");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_validate_api_key_success() {
        let mut mock_repo = MockApiKeyRepo::new();

        let user_uuid = Uuid::now_v7();
        let key_uuid = Uuid::now_v7();
        let api_key = ApiKey {
            uuid: key_uuid,
            user_uuid,
            key_hash: "hashed_key".to_string(),
            name: "Test Key".to_string(),
            description: Some("Test Description".to_string()),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            expires_at: None,
            last_used_at: None,
            created_by: user_uuid,
            published: true,
        };

        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("valid_key"))
            .returning(move |_| Ok(Some((api_key.clone(), user_uuid))));

        let service = ApiKeyService::new(Arc::new(mock_repo));
        let result = service.validate_api_key("valid_key").await;

        assert!(result.is_ok());
        let api_key_result = result.unwrap();
        assert!(api_key_result.is_some());
    }

    #[tokio::test]
    async fn test_validate_api_key_not_found() {
        let mut mock_repo = MockApiKeyRepo::new();

        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("invalid_key"))
            .returning(|_| Ok(None));

        let service = ApiKeyService::new(Arc::new(mock_repo));
        let result = service.validate_api_key("invalid_key").await;

        assert!(result.is_ok());
        let api_key_result = result.unwrap();
        assert!(api_key_result.is_none());
    }
}
