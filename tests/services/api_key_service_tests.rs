use async_trait::async_trait;
use mockall::{
    mock,
    predicate::{self, eq},
};
use r_data_core::{
    entity::admin_user::{ApiKey, ApiKeyRepositoryTrait},
    error::{Error, Result},
    services::ApiKeyService,
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

// Create our own mock for testing
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
async fn test_create_api_key_with_invalid_user_uuid() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let invalid_user_uuid = Uuid::now_v7();

    // Setup mock to simulate a foreign key constraint error
    mock_repo
        .expect_create_new_api_key()
        .returning(|_, _, _, _| {
            // Create a custom error message for the foreign key violation
            let error_message = "foreign key constraint violation".to_string();
            Err(Error::Database(sqlx::Error::Protocol(error_message)))
        });

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to create a key with the invalid user UUID
    let result = service
        .create_api_key("Test Key", "Test Description", invalid_user_uuid, 30)
        .await;

    // Verify failure
    assert!(result.is_err());
    match result {
        Err(Error::Database(e)) => {
            let err_string = e.to_string();
            assert!(
                err_string.contains("foreign key constraint"),
                "Expected foreign key constraint error, got: {}",
                err_string
            );
        }
        _ => panic!("Expected database error, got: {:?}", result),
    }

    Ok(())
}

#[tokio::test]
async fn test_validate_expired_api_key() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let _user_uuid = Uuid::now_v7();
    let _key_uuid = Uuid::now_v7();

    // Setup test data with an expired key (expiry date in the past)
    let now = OffsetDateTime::now_utc();
    let _yesterday = now - Duration::days(1);

    // Mock returns None for an expired key - simulating the repository's behavior
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq("expired_key"))
        .returning(|_| Ok(None));

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to validate the expired key
    let result = service.validate_api_key("expired_key").await?;

    // Verify we get None (not authenticated)
    assert!(result.is_none(), "Expired key should not authenticate");

    Ok(())
}

#[tokio::test]
async fn test_create_api_key_with_long_name() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let user_uuid = Uuid::now_v7();

    // Create an extremely long name (1000 characters)
    let long_name = "a".repeat(1000);

    // Setup mock to simulate a database validation error
    mock_repo
        .expect_create_new_api_key()
        .with(
            predicate::function(move |s: &str| s.len() == 1000 && s.chars().all(|c| c == 'a')),
            predicate::always(),
            predicate::always(),
            predicate::always(),
        )
        .returning(|_, _, _, _| {
            // Use a Protocol error instead of trying to construct a PgDatabaseError
            Err(Error::Database(sqlx::Error::Protocol(
                "ERROR: value too long for type character varying(255)".to_string(),
            )))
        });

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to create a key with the long name
    let result = service
        .create_api_key(&long_name, "Test Description", user_uuid, 30)
        .await;

    // Verify failure
    assert!(result.is_err());
    match result {
        Err(Error::Database(e)) => {
            let err_string = e.to_string();
            assert!(
                err_string.contains("too long"),
                "Expected 'too long' error, got: {}",
                err_string
            );
        }
        _ => panic!("Expected database error, got: {:?}", result),
    }

    Ok(())
}

#[tokio::test]
async fn test_validate_inactive_api_key() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let user_uuid = Uuid::now_v7();
    let key_uuid = Uuid::now_v7();

    // Setup an inactive API key
    let _inactive_key = ApiKey {
        uuid: key_uuid,
        user_uuid,
        key_hash: "hashed_key".to_string(),
        name: "Inactive Key".to_string(),
        description: Some("Test Description".to_string()),
        is_active: false, // Inactive!
        created_at: OffsetDateTime::now_utc(),
        expires_at: None,
        last_used_at: None,
        created_by: user_uuid,
        published: true,
    };

    // The repository layer should filter out inactive keys and return None
    mock_repo
        .expect_find_api_key_for_auth()
        .with(eq("inactive_key"))
        .returning(|_| Ok(None));

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to validate the inactive key
    let result = service.validate_api_key("inactive_key").await?;

    // Verify we get None (not authenticated)
    assert!(result.is_none(), "Inactive key should not authenticate");

    Ok(())
}

#[tokio::test]
async fn test_revoke_nonexistent_key() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let user_uuid = Uuid::now_v7();
    let nonexistent_key_uuid = Uuid::now_v7();

    // Setup mock to return None for the nonexistent key
    mock_repo
        .expect_get_by_uuid()
        .with(eq(nonexistent_key_uuid))
        .returning(|_| Ok(None));

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to revoke a nonexistent key
    let result = service.revoke_key(nonexistent_key_uuid, user_uuid).await;

    // Verify we get a NotFound error
    assert!(result.is_err());
    match result {
        Err(Error::NotFound(msg)) => {
            assert_eq!(msg, "API key not found");
        }
        _ => panic!("Expected NotFound error, got: {:?}", result),
    }

    Ok(())
}

#[tokio::test]
async fn test_revoke_key_unauthorized() -> Result<()> {
    let mut mock_repo = MockApiKeyRepo::new();
    let key_uuid = Uuid::now_v7();
    let key_owner_uuid = Uuid::now_v7();
    let different_user_uuid = Uuid::now_v7();

    // Setup a key that belongs to key_owner_uuid
    let api_key = ApiKey {
        uuid: key_uuid,
        user_uuid: key_owner_uuid, // This user owns the key
        key_hash: "hashed_key".to_string(),
        name: "Test Key".to_string(),
        description: Some("Test Description".to_string()),
        is_active: true,
        created_at: OffsetDateTime::now_utc(),
        expires_at: None,
        last_used_at: None,
        created_by: key_owner_uuid,
        published: true,
    };

    // Mock returns the key but it belongs to a different user
    mock_repo
        .expect_get_by_uuid()
        .with(eq(key_uuid))
        .returning(move |_| Ok(Some(api_key.clone())));

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to revoke the key as a different user
    let result = service.revoke_key(key_uuid, different_user_uuid).await;

    // Verify we get a Forbidden error
    assert!(result.is_err());
    match result {
        Err(Error::Forbidden(msg)) => {
            assert_eq!(msg, "You don't have permission to revoke this API key");
        }
        _ => panic!("Expected Forbidden error, got: {:?}", result),
    }

    Ok(())
}

#[tokio::test]
async fn test_negative_expiration_days() -> Result<()> {
    let mock_repo = MockApiKeyRepo::new();
    let user_uuid = Uuid::now_v7();

    let service = ApiKeyService::new(Arc::new(mock_repo));

    // Attempt to create a key with negative expiration days
    let result = service
        .create_api_key("Test Key", "Test Description", user_uuid, -10)
        .await;

    // Verify we get a validation error
    assert!(result.is_err());
    match result {
        Err(Error::Validation(msg)) => {
            assert_eq!(msg, "Expiration days cannot be negative");
        }
        _ => panic!("Expected Validation error, got: {:?}", result),
    }

    Ok(())
}
