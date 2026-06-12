#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `list_by_user` (sort variants, invalid order) and `count_by_user`
//! (unknown user returns 0).

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
// list_by_user — sort DESC
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_by_user_sort_name_desc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    repo.create_new_api_key("aaa_desc_key", "d", user_uuid, 30)
        .await?;
    repo.create_new_api_key("zzz_desc_key", "d", user_uuid, 30)
        .await?;

    let keys = repo
        .list_by_user(
            user_uuid,
            10,
            0,
            Some("name".to_string()),
            Some("DESC".to_string()),
        )
        .await?;

    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].name, "zzz_desc_key");
    assert_eq!(keys[1].name, "aaa_desc_key");

    Ok(())
}

// ---------------------------------------------------------------------------
// list_by_user — sort by last_used_at (NULLS LAST)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_by_user_sort_last_used_at_nulls_last() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_used, _) = repo
        .create_new_api_key(&random_string("used_key"), "d", user_uuid, 30)
        .await?;
    let (_key_unused, _) = repo
        .create_new_api_key(&random_string("unused_key"), "d", user_uuid, 30)
        .await?;

    // Touch last_used_at on key_used only
    repo.update_last_used(key_used).await?;

    let keys = repo
        .list_by_user(
            user_uuid,
            10,
            0,
            Some("last_used_at".to_string()),
            Some("ASC".to_string()),
        )
        .await?;

    assert_eq!(keys.len(), 2);
    // NULLS LAST — key with last_used_at set comes first in ASC
    assert_eq!(
        keys[0].uuid, key_used,
        "Key with last_used_at should sort before NULL"
    );
    assert!(
        keys[1].last_used_at.is_none(),
        "Key with NULL last_used_at should be last"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// list_by_user — sort by expires_at (NULLS LAST)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_by_user_sort_expires_at_nulls_last() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    // Key with expiry (10 days)
    let (key_expires, _) = repo
        .create_new_api_key(&random_string("exp_key"), "d", user_uuid, 10)
        .await?;
    // Key without expiry (expires_in_days = 0)
    let (_key_no_expiry, _) = repo
        .create_new_api_key(&random_string("no_exp_key"), "d", user_uuid, 0)
        .await?;

    let keys = repo
        .list_by_user(
            user_uuid,
            10,
            0,
            Some("expires_at".to_string()),
            Some("ASC".to_string()),
        )
        .await?;

    assert_eq!(keys.len(), 2);
    // NULLS LAST — key with expires_at set comes first
    assert_eq!(
        keys[0].uuid, key_expires,
        "Key with expires_at should sort before NULL"
    );
    assert!(
        keys[1].expires_at.is_none(),
        "Key with NULL expires_at should be last"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// list_by_user — invalid sort_order defaults to ASC
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_by_user_invalid_sort_order_defaults_to_asc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    repo.create_new_api_key("yyy_invalid_order", "d", user_uuid, 30)
        .await?;
    repo.create_new_api_key("bbb_invalid_order", "d", user_uuid, 30)
        .await?;

    // "RANDOM" is not "ASC" or "DESC" — must fall back to ASC
    let keys = repo
        .list_by_user(
            user_uuid,
            10,
            0,
            Some("name".to_string()),
            Some("RANDOM".to_string()),
        )
        .await?;

    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].name, "bbb_invalid_order");
    assert_eq!(keys[1].name, "yyy_invalid_order");

    Ok(())
}

// ---------------------------------------------------------------------------
// count_by_user — returns 0 for completely unknown user UUID
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_count_by_user_zero_for_unknown_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let unknown = Uuid::now_v7();

    let count = repo.count_by_user(unknown).await?;
    assert_eq!(count, 0, "Unknown user must have count 0");

    Ok(())
}
