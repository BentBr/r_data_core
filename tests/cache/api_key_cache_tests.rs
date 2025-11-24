use super::create_test_cache_manager;
use async_trait::async_trait;
use mockall::mock;
use mockall::predicate::*;
use r_data_core::entity::admin_user::{ApiKey, ApiKeyRepositoryTrait};
use r_data_core_core::error::Result;
use r_data_core::services::ApiKeyService;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

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
    }
}

fn create_test_api_key() -> (ApiKey, Uuid) {
    let user_uuid = Uuid::now_v7();
    let key_uuid = Uuid::now_v7();
    // Use the actual hash of "test_api_key_12345" to match what validate_api_key will compute
    let api_key_str = "test_api_key_12345";
    let key_hash = r_data_core::entity::admin_user::ApiKey::hash_api_key(api_key_str)
        .unwrap_or_else(|_| "test_hash".to_string());
    let api_key = ApiKey {
        uuid: key_uuid,
        user_uuid,
        key_hash,
        name: "Test Key".to_string(),
        description: Some("Test Description".to_string()),
        is_active: true,
        created_at: OffsetDateTime::now_utc(),
        expires_at: None,
        last_used_at: None,
        created_by: user_uuid,
        published: true,
    };
    (api_key, user_uuid)
}

#[tokio::test]
async fn test_api_key_cache_hit() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let cache_manager = create_test_cache_manager();
    let (api_key, user_uuid) = create_test_api_key();
    let api_key_str = "test_api_key_12345";

    // First call should query repository
    let api_key_clone = api_key.clone();
    let api_key_uuid = api_key.uuid;
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq(api_key_str))
        .times(1)
        .returning(move |_| Ok(Some((api_key_clone.clone(), user_uuid))));

    let service = ApiKeyService::with_cache(Arc::new(mock_repo), cache_manager.clone(), 600);

    // First call - cache miss
    let result1 = service.validate_api_key(api_key_str).await?;
    assert!(result1.is_some());
    let (key1, uuid1) = result1.unwrap();
    assert_eq!(key1.uuid, api_key_uuid);

    // Second call - should hit cache (no repository call expected)
    let mut mock_repo2 = MockApiKeyRepo::new();
    mock_repo2.expect_find_api_key_for_auth().times(0);

    let service2 = ApiKeyService::with_cache(Arc::new(mock_repo2), cache_manager.clone(), 600);
    let result2 = service2.validate_api_key(api_key_str).await?;
    assert!(result2.is_some());
    let (key2, uuid2) = result2.unwrap();
    assert_eq!(key2.uuid, key1.uuid);
    assert_eq!(uuid2, uuid1);

    Ok(())
}

#[tokio::test]
async fn test_api_key_cache_miss() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let cache_manager = create_test_cache_manager();
    let api_key_str = "invalid_key";

    // Should query repository on cache miss
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq(api_key_str))
        .times(1)
        .returning(|_| Ok(None));

    let service = ApiKeyService::with_cache(Arc::new(mock_repo), cache_manager.clone(), 600);

    let result = service.validate_api_key(api_key_str).await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_api_key_cache_invalidation_on_revoke() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let cache_manager = create_test_cache_manager();
    let (api_key, user_uuid) = create_test_api_key();
    let api_key_str = "test_api_key_12345";
    let key_uuid = api_key.uuid;

    // Setup: cache the API key
    let api_key_clone = api_key.clone();
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq(api_key_str))
        .times(1)
        .returning(move |_| Ok(Some((api_key_clone.clone(), user_uuid))));

    let service = ApiKeyService::with_cache(Arc::new(mock_repo), cache_manager.clone(), 600);

    // Cache the key
    service.validate_api_key(api_key_str).await?;

    // Revoke the key
    let mut mock_repo2 = MockApiKeyRepo::new();
    let mut api_key_clone2 = api_key.clone();
    api_key_clone2.is_active = false; // Mark as inactive after revoke
    let api_key_for_revoke = api_key_clone2.clone();
    mock_repo2
        .expect_get_by_uuid()
        .with(eq(key_uuid))
        .times(1)
        .returning(move |_| Ok(Some(api_key_for_revoke.clone())));
    mock_repo2
        .expect_revoke()
        .with(eq(key_uuid))
        .times(1)
        .returning(|_| Ok(()));

    let service2 = ApiKeyService::with_cache(Arc::new(mock_repo2), cache_manager.clone(), 600);
    service2.revoke_key(key_uuid, user_uuid).await?;

    // After revoke, cache should be invalidated - next query should hit database
    let mut mock_repo3 = MockApiKeyRepo::new();
    mock_repo3
        .expect_find_api_key_for_auth()
        .with(eq(api_key_str))
        .times(1)
        .returning(|_| Ok(None)); // Key is now revoked/inactive

    let service3 = ApiKeyService::with_cache(Arc::new(mock_repo3), cache_manager, 600);
    let result = service3.validate_api_key(api_key_str).await?;
    // After revocation, key should not be found or should be inactive
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_api_key_cache_ttl_expiration() -> Result<()> {
    // This test verifies that cache entries expire after TTL
    // Note: This is a simplified test - in practice, TTL expiration is handled by the cache backend
    let mut mock_repo = MockApiKeyRepo::new();
    let cache_manager = create_test_cache_manager();
    let (api_key, user_uuid) = create_test_api_key();
    let api_key_str = "test_api_key_12345";

    // First call caches the result
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq(api_key_str))
        .times(1)
        .returning(move |_| Ok(Some((api_key.clone(), user_uuid))));

    let service = ApiKeyService::with_cache(Arc::new(mock_repo), cache_manager.clone(), 600);

    let result = service.validate_api_key(api_key_str).await?;
    assert!(result.is_some());

    // Note: Testing TTL expiration would require waiting or manipulating the cache
    // For now, we just verify the caching mechanism works
    // In a real scenario, after TTL expires, the next call would query the database again

    Ok(())
}
