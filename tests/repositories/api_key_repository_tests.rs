use crate::common;
use r_data_core::{
    entity::admin_user::{ApiKeyRepository, ApiKeyRepositoryTrait},
    error::Result,
};
use std::sync::Arc;

#[tokio::test]
async fn test_create_and_find_api_key() -> Result<()> {
    // Setup and making sure to only work in transactions
    let pool = common::setup_test_db().await;
    pool.begin().await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let name = common::random_string("test_key");

    let user_uuid = common::create_test_admin_user(&pool).await?;

    // Create a new key
    let (key_uuid, key_value) = repo
        .create_new_api_key(&name, "Test key for integration tests", user_uuid, 30)
        .await?;

    // Find the key we just created
    let found_key = repo.get_by_uuid(key_uuid).await?;

    // Verify
    assert!(found_key.is_some());
    let key = found_key.unwrap();
    assert_eq!(key.uuid, key_uuid);
    assert_eq!(key.user_uuid, user_uuid);
    assert_eq!(key.name, name);

    // Test finding by the actual key value
    let auth_result = repo.find_api_key_for_auth(&key_value).await?;
    assert!(auth_result.is_some());
    let (found_api_key, found_user_uuid) = auth_result.unwrap();
    assert_eq!(found_api_key.uuid, key_uuid);
    assert_eq!(found_user_uuid, user_uuid);

    // Clean up - revoke the key
    repo.revoke(key_uuid).await?;

    // Verify it was revoked
    let revoked_key = repo.get_by_uuid(key_uuid).await?;
    assert!(revoked_key.is_some());
    assert!(!revoked_key.unwrap().is_active);

    Ok(())
}
