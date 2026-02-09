#![allow(clippy::unwrap_used)]

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
        async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64, sort_by: Option<String>, sort_order: Option<String>) -> Result<Vec<ApiKey>>;
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
        async fn get_api_key_roles(&self, api_key_uuid: Uuid) -> Result<Vec<Uuid>>;
        async fn assign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()>;
        async fn unassign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()>;
        async fn update_api_key_roles(&self, api_key_uuid: Uuid, role_uuids: &[Uuid]) -> Result<()>;
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
