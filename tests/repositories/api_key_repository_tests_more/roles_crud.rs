#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Tests for role operations, `update_api_key_roles` with empty slice,
//! `unassign_role` no-op, `create` FK failure, `create_new_api_key` expiry,
//! and `reassign` to same user.

use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Insert a role into the test DB and return its UUID.
///
/// # Panics
/// Panics if the database insert fails.
pub async fn create_test_role(pool: &PgPool, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO roles (name, created_by, published) VALUES ($1, $2, true) RETURNING uuid",
    )
    .bind(random_string("role"))
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("failed to create test role")
}

// ---------------------------------------------------------------------------
// update_api_key_roles — empty slice clears all roles
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_update_api_key_roles_empty_slice_clears_all() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("clear_roles_key"), "d", user_uuid, 30)
        .await?;

    let role = create_test_role(&pool.pool, user_uuid).await;
    repo.assign_role(key_uuid, role).await?;

    let before = repo.get_api_key_roles(key_uuid).await?;
    assert_eq!(before.len(), 1);

    // Updating with empty slice should remove all
    repo.update_api_key_roles(key_uuid, &[]).await?;

    let after = repo.get_api_key_roles(key_uuid).await?;
    assert!(after.is_empty(), "All roles should be cleared");

    Ok(())
}

// ---------------------------------------------------------------------------
// unassign_role — removing a role that was never assigned is a no-op
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_unassign_role_nonexistent_is_noop() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("noop_unassign_key"), "d", user_uuid, 30)
        .await?;

    let role = create_test_role(&pool.pool, user_uuid).await;

    // Unassign a role that was never assigned — must not error
    let result = repo.unassign_role(key_uuid, role).await;
    assert!(result.is_ok(), "Unassigning non-existent role must succeed");

    let roles = repo.get_api_key_roles(key_uuid).await?;
    assert!(roles.is_empty());

    Ok(())
}

// ---------------------------------------------------------------------------
// create (direct) — FK violation returns error
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_create_direct_fk_violation_returns_error() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let nonexistent_user = Uuid::now_v7();

    let key_value = ApiKey::generate_key();
    let key_hash = ApiKey::hash_api_key(&key_value)?;

    let api_key = ApiKey {
        uuid: Uuid::now_v7(),
        user_uuid: nonexistent_user,
        key_hash,
        name: "fk_fail_direct".to_string(),
        description: None,
        is_active: true,
        created_at: OffsetDateTime::now_utc(),
        expires_at: None,
        last_used_at: None,
        created_by: nonexistent_user,
        published: true,
    };

    let result = repo.create(&api_key).await;
    assert!(
        result.is_err(),
        "Creating a key for a non-existent user must fail"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// create_new_api_key — positive expiry stores a future expires_at
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_create_new_api_key_positive_expiry_stored() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let before = OffsetDateTime::now_utc();

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("expiry_check"), "d", user_uuid, 7)
        .await?;

    let key = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert!(key.expires_at.is_some(), "expires_at must be set");

    let exp = key.expires_at.unwrap();
    // Must be at least 6 days from the moment before creation (avoids clock jitter)
    let min_expiry = before + time::Duration::days(6);
    assert!(
        exp > min_expiry,
        "expires_at {exp} must be at least 6 days in the future"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// reassign — reassigning to the same user is a no-op (succeeds, user unchanged)
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_reassign_to_same_user_is_noop() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key_uuid, _) = repo
        .create_new_api_key(&random_string("same_user_reassign"), "d", user_uuid, 30)
        .await?;

    repo.reassign(key_uuid, user_uuid).await?;

    let key = repo.get_by_uuid(key_uuid).await?.unwrap();
    assert_eq!(key.user_uuid, user_uuid, "Owner must be unchanged");

    Ok(())
}
