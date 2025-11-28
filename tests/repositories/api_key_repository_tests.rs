use crate::common::utils;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use serial_test::serial;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_create_and_find_api_key() -> Result<()> {
    // Setup and making sure to only work in transactions
    let pool = utils::setup_test_db().await;
    utils::clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let name = utils::random_string("test_key");

    let user_uuid = utils::create_test_admin_user(&pool).await?;

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

#[tokio::test]
#[serial]
async fn test_create_api_key_with_non_existent_user() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let name = utils::random_string("test_key");

    // Generate a random UUID that doesn't exist in the database
    // Use a different timestamp for v7 to ensure it doesn't exist
    // Subtracting a large number of seconds to ensure it's far in the past
    let non_existent_uuid = Uuid::now_v7();

    // Verify this UUID doesn't exist in the database
    let user_exists = sqlx::query!(
        "SELECT COUNT(*) as count FROM admin_users WHERE uuid = $1",
        non_existent_uuid
    )
    .fetch_one(&pool)
    .await?
    .count
    .unwrap_or(0)
        > 0;

    assert!(!user_exists, "Test UUID should not exist in the database");

    // Attempt to create key with non-existent user UUID
    let result = repo
        .create_new_api_key(&name, "Test key with invalid user", non_existent_uuid, 30)
        .await;

    // Verify the operation fails with a foreign key constraint error
    assert!(result.is_err());
    match result {
        Err(r_data_core_core::error::Error::Database(db_error)) => {
            // Verify it's a foreign key constraint violation
            assert!(
                db_error.to_string().contains("foreign key constraint"),
                "Expected foreign key constraint error, got: {}",
                db_error
            );
        }
        Err(other) => panic!("Expected database error, got: {:?}", other),
        Ok(_) => panic!("Expected error but operation succeeded"),
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_api_key_last_used_update() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let name = utils::random_string("test_key");

    let user_uuid = utils::create_test_admin_user(&pool).await?;

    // Create a new key
    let (key_uuid, key_value) = repo
        .create_new_api_key(&name, "Test key for last_used tracking", user_uuid, 30)
        .await?;

    // Verify the initial state-last_used_at should be None
    let initial_key = repo.get_by_uuid(key_uuid).await?;
    assert!(initial_key.is_some(), "Key should exist after creation");
    let initial_key = initial_key.unwrap();
    assert!(
        initial_key.last_used_at.is_none(),
        "New key should have null last_used_at"
    );

    // Use the key for authentication (which should update last_used_at)
    let auth_result = repo.find_api_key_for_auth(&key_value).await?;
    assert!(
        auth_result.is_some(),
        "Key should be valid for authentication"
    );

    // Small delay to ensure the timestamp change is detectable
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Verify the key's last_used_at was updated
    let updated_key = repo.get_by_uuid(key_uuid).await?;
    assert!(
        updated_key.is_some(),
        "Key should still exist after authentication"
    );
    let updated_key = updated_key.unwrap();
    assert!(
        updated_key.last_used_at.is_some(),
        "Key should have last_used_at timestamp after authentication"
    );

    // Use it again and verify timestamp changes
    let first_used_at = updated_key.last_used_at.unwrap();

    // Small delay to ensure the timestamp change is detectable
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Use the key again
    let auth_result2 = repo.find_api_key_for_auth(&key_value).await?;
    assert!(
        auth_result2.is_some(),
        "Key should be valid for second authentication"
    );

    // Verify timestamp updated
    let latest_key = repo.get_by_uuid(key_uuid).await?;
    assert!(
        latest_key.is_some(),
        "Key should still exist after second authentication"
    );
    let latest_key = latest_key.unwrap();
    assert!(
        latest_key.last_used_at.is_some(),
        "Key should have last_used_at timestamp after second authentication"
    );
    let latest_used_at = latest_key.last_used_at.unwrap();

    assert!(
        latest_used_at > first_used_at,
        "last_used_at should be updated: first={:?}, latest={:?}",
        first_used_at,
        latest_used_at
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_expired_api_key() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.clone()));
    let name = utils::random_string("expired_key");

    let user_uuid = utils::create_test_admin_user(&pool).await?;

    // Create a key that expired yesterday
    let one_day_ago = OffsetDateTime::now_utc() - Duration::days(1);

    // Insert directly with SQL to bypass the normal creation logic
    let key_uuid = Uuid::now_v7();
    let key_value = "test_expired_key_value";
    let key_hash = r_data_core_core::admin_user::ApiKey::hash_api_key(key_value)?;

    sqlx::query!(
        r#"
        INSERT INTO api_keys 
        (uuid, user_uuid, key_hash, name, description, is_active, created_at, expires_at, created_by, published)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        key_uuid,
        user_uuid,
        key_hash,
        name,
        Some("Expired test key"),
        true,  // Active
        one_day_ago - Duration::days(30), // Created 31 days ago
        one_day_ago, // Expired yesterday
        user_uuid,
        true
    )
    .execute(&pool)
    .await?;

    // Attempt to authenticate with the expired key
    let auth_result = repo.find_api_key_for_auth(key_value).await?;

    // Verify the expired key is not authenticated
    assert!(auth_result.is_none(), "Expired key should not authenticate");

    // Verify we can still retrieve the key directly
    let expired_key = repo.get_by_uuid(key_uuid).await?;
    assert!(
        expired_key.is_some(),
        "Key should be retrievable for uuid {}",
        key_uuid
    );
    let key = expired_key.unwrap();
    assert!(key.expires_at.is_some());
    assert!(
        key.expires_at.unwrap() < OffsetDateTime::now_utc(),
        "Key expiration should be in the past"
    );

    Ok(())
}
