#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Round-trips for the inherent `AdminUserRepository` operations backing the
//! `user_actions` console binary: `set_active` and `reset_password`.

use r_data_core_core::admin_user::UserStatus;
use r_data_core_core::error::Result;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_set_active_toggles_flag() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let uuid = create_test_admin_user(&pool.pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

    repo.set_active(&uuid, false).await?;
    assert!(
        !repo.find_by_uuid(&uuid).await?.unwrap().is_active,
        "deactivate → is_active false"
    );

    repo.set_active(&uuid, true).await?;
    assert!(
        repo.find_by_uuid(&uuid).await?.unwrap().is_active,
        "activate → is_active true"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_reset_password_sets_hash_and_clears_lockout() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let uuid = create_test_admin_user(&pool.pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));

    // Lock the account first, then prove a password reset also clears the lock.
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5)
        .await?;
    assert_eq!(
        repo.find_by_uuid(&uuid).await?.unwrap().status,
        UserStatus::Locked
    );

    let new_hash =
        "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$WwD1am7XJrm2JAMuY4QQVGRBfFmLwUJX7p4NCZEw9MU";
    repo.reset_password(&uuid, new_hash).await?;

    let user = repo.find_by_uuid(&uuid).await?.unwrap();
    assert_eq!(user.password_hash, new_hash, "password_hash updated");
    assert_eq!(user.status, UserStatus::Active, "lock cleared on reset");
    assert_eq!(user.failed_login_attempts, 0, "counter cleared on reset");

    clear_test_db(&pool).await?;
    Ok(())
}
