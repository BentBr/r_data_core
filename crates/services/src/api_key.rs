use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_persistence::{ApiKeyRepository, ApiKeyRepositoryTrait};
use std::sync::Arc;
use uuid::Uuid;

/// Service for handling API key operations
pub struct ApiKeyService {
    repository: Arc<dyn ApiKeyRepositoryTrait>,
    cache_manager: Option<Arc<CacheManager>>,
    api_key_ttl: u64,
}

impl ApiKeyService {
    /// Create a new API key service with a concrete repository
    pub fn new(repository: Arc<dyn ApiKeyRepositoryTrait>) -> Self {
        Self {
            repository,
            cache_manager: None,
            api_key_ttl: 600, // Default 10 minutes
        }
    }

    /// Create a new API key service with cache manager
    pub fn with_cache(
        repository: Arc<dyn ApiKeyRepositoryTrait>,
        cache_manager: Arc<CacheManager>,
        api_key_ttl: u64,
    ) -> Self {
        Self {
            repository,
            cache_manager: Some(cache_manager),
            api_key_ttl,
        }
    }

    /// Create a new API key service from a concrete repository
    #[must_use]
    pub fn from_repository(repository: ApiKeyRepository) -> Self {
        Self {
            repository: Arc::new(repository),
            cache_manager: None,
            api_key_ttl: 600, // Default 10 minutes
        }
    }

    /// Generate cache key for API key by hash
    fn cache_key_by_hash(key_hash: &str) -> String {
        format!("api_key:hash:{key_hash}")
    }

    /// Create a new API key
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn create_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        // Validation
        if name.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "API key name cannot be empty".to_string(),
            ));
        }

        if expires_in_days < 0 {
            return Err(r_data_core_core::error::Error::Validation(
                "Expiration days cannot be negative".to_string(),
            ));
        }

        self.repository
            .create_new_api_key(name, description, created_by, expires_in_days)
            .await
    }

    /// Validate an API key and return user information if valid
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn validate_api_key(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>> {
        if api_key.is_empty() {
            return Ok(None);
        }

        // Hash the API key for cache lookup
        let key_hash = match ApiKey::hash_api_key(api_key) {
            Ok(hash) => hash,
            Err(e) => {
                log::warn!("Failed to hash API key for cache: {e}");
                // Fall back to repository lookup
                return self.repository.find_api_key_for_auth(api_key).await;
            }
        };

        // Check cache first if cache manager is available
        if let Some(cache) = &self.cache_manager {
            let cache_key = Self::cache_key_by_hash(&key_hash);
            if let Ok(Some(cached)) = cache.get::<(ApiKey, Uuid)>(&cache_key).await {
                // Cache hit - return cached result (skip last_used_at update for performance)
                return Ok(Some(cached));
            }
        }

        // Cache miss - query repository
        let result = self.repository.find_api_key_for_auth(api_key).await?;

        // Cache the result if valid and cache manager is available
        if let Some((ref key, ref user_uuid)) = result {
            if let Some(cache) = &self.cache_manager {
                let cache_key = Self::cache_key_by_hash(&key_hash);
                // Use configured TTL (0 = no expiration, but we use Some() to respect TTL)
                let ttl = if self.api_key_ttl > 0 {
                    Some(self.api_key_ttl)
                } else {
                    None // No expiration
                };
                if let Err(e) = cache.set(&cache_key, &(key.clone(), *user_uuid), ttl).await {
                    log::warn!("Failed to cache API key validation result: {e}");
                }
            }
        }

        Ok(result)
    }

    /// List all API keys for a user
    ///
    /// # Errors
    /// Returns an error if the database query fails
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
    ///
    /// # Errors
    /// Returns an error if the key is not found, user doesn't have permission, or database operation fails
    pub async fn revoke_key(&self, key_uuid: Uuid, user_uuid: Uuid) -> Result<()> {
        // Verify ownership first
        let key = self.repository.get_by_uuid(key_uuid).await?;

        match key {
            Some(key) if key.user_uuid == user_uuid => {
                // Revoke the key
                let result = self.repository.revoke(key_uuid).await;

                // Invalidate cache if revocation succeeded
                if result.is_ok() {
                    if let Some(cache) = &self.cache_manager {
                        let cache_key = Self::cache_key_by_hash(&key.key_hash);
                        if let Err(e) = cache.delete(&cache_key).await {
                            log::warn!("Failed to invalidate API key cache: {e}");
                        }
                    }
                }

                result
            }
            Some(_) => Err(r_data_core_core::error::Error::Forbidden(
                "You don't have permission to revoke this API key".to_string(),
            )),
            None => Err(r_data_core_core::error::Error::NotFound(
                "API key not found".to_string(),
            )),
        }
    }

    /// Get a key by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_key(&self, key_uuid: Uuid) -> Result<Option<ApiKey>> {
        self.repository.get_by_uuid(key_uuid).await
    }

    /// Reassign an API key to a different user
    ///
    /// # Errors
    /// Returns an error if the key is not found or database operation fails
    pub async fn reassign_key(&self, key_uuid: Uuid, new_user_uuid: Uuid) -> Result<()> {
        // Verify the key exists
        let key = self.get_key(key_uuid).await?;
        if key.is_none() {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "API key with UUID {key_uuid} not found"
            )));
        }

        // Reassign the key
        self.repository.reassign(key_uuid, new_user_uuid).await
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
            async fn count_by_user(&self, user_uuid: Uuid) -> Result<i64>;
            async fn get_api_key_permission_schemes(&self, api_key_uuid: Uuid) -> Result<Vec<Uuid>>;
            async fn assign_permission_scheme(&self, api_key_uuid: Uuid, scheme_uuid: Uuid) -> Result<()>;
            async fn unassign_permission_scheme(&self, api_key_uuid: Uuid, scheme_uuid: Uuid) -> Result<()>;
            async fn update_api_key_schemes(&self, api_key_uuid: Uuid, scheme_uuids: &[Uuid]) -> Result<()>;
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
            .returning(move |_, _, _, _| Ok((key_uuid, (*key_value).to_string())));

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
            Err(r_data_core_core::error::Error::Validation(msg)) => {
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

    /// Unit test for reassign functionality
    #[tokio::test]
    async fn test_reassign_key() {
        let mut mock_repo = MockApiKeyRepo::new();
        let key_uuid = Uuid::now_v7();
        let original_user_uuid = Uuid::now_v7();
        let new_user_uuid = Uuid::now_v7();

        // Create mock API key with original user
        let api_key = ApiKey {
            uuid: key_uuid,
            user_uuid: original_user_uuid,
            key_hash: "hashed_key".to_string(),
            name: "Test Key".to_string(),
            description: Some("Test Description".to_string()),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            expires_at: None,
            last_used_at: None,
            created_by: original_user_uuid,
            published: true,
        };

        // Create mock API key with new user_uuid after reassignment
        let reassigned_api_key = ApiKey {
            uuid: key_uuid,
            user_uuid: new_user_uuid, // New user
            key_hash: "hashed_key".to_string(),
            name: "Test Key".to_string(),
            description: Some("Test Description".to_string()),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            expires_at: None,
            last_used_at: None,
            created_by: original_user_uuid,
            published: true,
        };

        // Setup mock to return original key first, then reassigned key
        mock_repo
            .expect_get_by_uuid()
            .with(eq(key_uuid))
            .returning(move |_| Ok(Some(api_key.clone())))
            .times(1);

        // Mock successful reassignment
        mock_repo
            .expect_reassign()
            .with(eq(key_uuid), eq(new_user_uuid))
            .returning(|_, _| Ok(()));

        // Mock returns the reassigned key after reassignment
        mock_repo
            .expect_get_by_uuid()
            .with(eq(key_uuid))
            .returning(move |_| Ok(Some(reassigned_api_key.clone())))
            .times(1);

        let service = ApiKeyService::new(Arc::new(mock_repo));

        // Reassign the key
        let result = service.reassign_key(key_uuid, new_user_uuid).await;
        assert!(result.is_ok(), "Key reassignment should succeed");

        // Verify the key has been reassigned
        let key_after_reassign = service.get_key(key_uuid).await.unwrap();
        assert!(
            key_after_reassign.is_some(),
            "Key should exist after reassignment"
        );

        let key = key_after_reassign.unwrap();
        assert_eq!(
            key.user_uuid, new_user_uuid,
            "Key should be assigned to the new user"
        );
        assert_ne!(
            key.user_uuid, original_user_uuid,
            "Key should no longer be assigned to original user"
        );
    }

    /// Unit test for reassigning a nonexistent key
    #[tokio::test]
    async fn test_reassign_nonexistent_key() {
        let mut mock_repo = MockApiKeyRepo::new();
        let nonexistent_key_uuid = Uuid::now_v7();
        let new_user_uuid = Uuid::now_v7();

        // Mock returns None for the nonexistent key
        mock_repo
            .expect_get_by_uuid()
            .with(eq(nonexistent_key_uuid))
            .returning(|_| Ok(None));

        let service = ApiKeyService::new(Arc::new(mock_repo));

        // Attempt to reassign a nonexistent key
        let result = service
            .reassign_key(nonexistent_key_uuid, new_user_uuid)
            .await;

        // Verify we get a NotFound error
        assert!(result.is_err());
        match result {
            Err(r_data_core_core::error::Error::NotFound(msg)) => {
                assert!(
                    msg.contains("not found"),
                    "Expected 'not found' in error message"
                );
            }
            _ => panic!("Expected NotFound error, got: {result:?}"),
        }
    }
}
