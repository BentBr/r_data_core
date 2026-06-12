#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `get_by_uuid`, `find_api_key_for_auth`, and `get_by_hash`
//! covering not-found and inactive-key branches.

use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// get_by_uuid — explicit not-found
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_by_uuid_returns_none_for_unknown() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let unknown = Uuid::now_v7();

    let result = repo.get_by_uuid(unknown).await?;
    assert!(result.is_none(), "Unknown UUID must return None");

    Ok(())
}

// ---------------------------------------------------------------------------
// find_api_key_for_auth — inactive (not expired) key is rejected
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_find_api_key_for_auth_inactive_key_rejected() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, key_value) = repo
        .create_new_api_key(&random_string("inactive_key"), "d", user_uuid, 30)
        .await?;

    // Confirm active before revoke
    let before = repo.find_api_key_for_auth(&key_value).await?;
    assert!(before.is_some(), "Key should be valid before revoke");

    // Revoke (sets is_active = false, no expiry change)
    repo.revoke(key_uuid).await?;

    // The key exists in the DB but is_active = false
    let db_key = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert!(!db_key.is_active);

    // Auth must reject it
    let after = repo.find_api_key_for_auth(&key_value).await?;
    assert!(
        after.is_none(),
        "Inactive key must not authenticate even when not expired"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// get_by_hash — inactive key is not returned
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_by_hash_inactive_key_returns_none() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, key_value) = repo
        .create_new_api_key(&random_string("hash_inactive"), "d", user_uuid, 30)
        .await?;

    // Active — should be found
    let found = repo.get_by_hash(&key_value).await?;
    assert!(found.is_some(), "Active key must be found by hash");

    repo.revoke(key_uuid).await?;

    // Inactive — must not be returned
    let after_revoke = repo.get_by_hash(&key_value).await?;
    assert!(
        after_revoke.is_none(),
        "Inactive key must not be returned by get_by_hash"
    );

    Ok(())
}
