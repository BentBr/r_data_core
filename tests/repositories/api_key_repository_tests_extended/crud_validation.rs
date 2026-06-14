#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_get_by_name() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;
    let key_name = random_string("named_key");

    let (key_uuid, _) = repo
        .create_new_api_key(&key_name, "desc", user_uuid, 30)
        .await?;

    let found = repo.get_by_name(user_uuid, &key_name).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().uuid, key_uuid);

    let not_found = repo.get_by_name(user_uuid, "does-not-exist").await?;
    assert!(not_found.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_by_hash() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, key_value) = repo
        .create_new_api_key(&random_string("hash_key"), "desc", user_uuid, 30)
        .await?;

    let found = repo.get_by_hash(&key_value).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().uuid, key_uuid);

    let not_found = repo.get_by_hash("not-a-real-key-value").await?;
    assert!(not_found.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_create_new_api_key_validation_empty_name() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let result = repo.create_new_api_key("", "desc", user_uuid, 30).await;
    assert!(result.is_err(), "Empty name should return an error");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_create_new_api_key_validation_whitespace_name() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let result = repo.create_new_api_key("   ", "desc", user_uuid, 30).await;
    assert!(
        result.is_err(),
        "Whitespace-only name should return an error"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_create_new_api_key_validation_negative_days() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let result = repo
        .create_new_api_key("valid_name", "desc", user_uuid, -1)
        .await;
    assert!(
        result.is_err(),
        "Negative expiry days should return an error"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_create_new_api_key_zero_days_no_expiry() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _key_value) = repo
        .create_new_api_key(&random_string("no_expiry"), "desc", user_uuid, 0)
        .await?;

    let key = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert!(
        key.expires_at.is_none(),
        "expires_in_days=0 should set no expiry"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_reassign_api_key() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user1 = create_test_admin_user(&pool).await?;
    let user2 = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("reassign_key"), "desc", user1, 30)
        .await?;

    let key_before = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert_eq!(key_before.user_uuid, user1);

    repo.reassign(key_uuid, user2).await?;

    let key_after = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert_eq!(key_after.user_uuid, user2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_update_last_used() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("last_used_key"), "desc", user_uuid, 30)
        .await?;

    let key = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert!(key.last_used_at.is_none());

    repo.update_last_used(key_uuid).await?;

    let key_after = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert!(key_after.last_used_at.is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_create_api_key_directly() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;
    let key_value = ApiKey::generate_key();
    let key_hash = ApiKey::hash_api_key(&key_value)?;

    let api_key = ApiKey {
        uuid: Uuid::now_v7(),
        user_uuid,
        key_hash,
        name: "direct_create".to_string(),
        description: Some("direct".to_string()),
        is_active: true,
        created_at: OffsetDateTime::now_utc(),
        expires_at: None,
        last_used_at: None,
        created_by: user_uuid,
        published: true,
    };
    let uuid = api_key.uuid;

    let created_uuid = repo.create(&api_key).await?;
    assert_eq!(created_uuid, uuid);

    let found = repo.get_by_uuid(uuid).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "direct_create");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_revoked_key_not_found_by_auth() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, key_value) = repo
        .create_new_api_key(&random_string("revoke_auth_key"), "d", user_uuid, 30)
        .await?;

    let before = repo.find_api_key_for_auth(&key_value).await?;
    assert!(before.is_some());

    repo.revoke(key_uuid).await?;

    let after = repo.find_api_key_for_auth(&key_value).await?;
    assert!(after.is_none());

    Ok(())
}
