#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Persistence of the `locked_until` column: written by `update_lockout_state`,
//! carried by `update_admin_user`, and round-tripped through `FromRow`.

use r_data_core_core::admin_user::UserStatus;
use r_data_core_core::error::Result;
use r_data_core_persistence::AdminUserRepositoryTrait;
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;
use time::{Duration, OffsetDateTime};

use super::users::{make_repo, seed_user};

/// A timestamp within a second of the expected one; Postgres stores microseconds.
fn approximately(actual: OffsetDateTime, expected: OffsetDateTime) -> bool {
    (actual - expected).abs() < Duration::seconds(1)
}

#[tokio::test]
#[serial]
async fn test_lockout_expiry_round_trips() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    let until = OffsetDateTime::now_utc() + Duration::minutes(15);
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5, Some(until))
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Locked);
    assert_eq!(user.failed_login_attempts, 5);
    assert!(user.locked_until.is_some_and(|t| approximately(t, until)));
    Ok(())
}

/// Unlocking must clear the expiry, otherwise a stale timestamp would linger on
/// an active account.
#[tokio::test]
#[serial]
async fn test_unlocking_clears_the_expiry() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    let until = OffsetDateTime::now_utc() + Duration::minutes(15);
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5, Some(until))
        .await?;
    repo.update_lockout_state(&uuid, &UserStatus::Active, 0, None)
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Active);
    assert_eq!(user.failed_login_attempts, 0);
    assert!(user.locked_until.is_none());
    Ok(())
}

/// An operator lock stores no expiry, which is what makes it permanent.
#[tokio::test]
#[serial]
async fn test_operator_lock_stores_no_expiry() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5, None)
        .await?;

    let user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Locked);
    assert!(user.locked_until.is_none());
    Ok(())
}

/// `update_admin_user` is the path the admin API takes, so it has to carry the
/// status and the lockout columns — not only the profile fields.
#[tokio::test]
#[serial]
async fn test_update_admin_user_persists_status_and_expiry() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    let mut user = repo.find_by_uuid(&uuid).await?.expect("user must exist");

    user.record_login_failure(1, 900);
    repo.update_admin_user(&user).await?;

    let locked = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(locked.status, UserStatus::Locked);
    assert_eq!(locked.failed_login_attempts, 1);
    assert!(locked.locked_until.is_some());

    // And the same path must be able to unlock again.
    let mut unlocked = locked;
    unlocked.set_status(UserStatus::Active);
    repo.update_admin_user(&unlocked).await?;

    let reloaded = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(reloaded.status, UserStatus::Active);
    assert_eq!(reloaded.failed_login_attempts, 0);
    assert!(reloaded.locked_until.is_none());
    Ok(())
}

/// A lockout already in the past is stored verbatim; releasing it is the
/// application's job, not the repository's.
#[tokio::test]
#[serial]
async fn test_past_expiry_is_stored_as_written() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    let uuid = seed_user(&repo, &pool).await?;
    let past = OffsetDateTime::now_utc() - Duration::hours(1);
    repo.update_lockout_state(&uuid, &UserStatus::Locked, 5, Some(past))
        .await?;

    let mut user = repo.find_by_uuid(&uuid).await?.expect("user must exist");
    assert_eq!(user.status, UserStatus::Locked);
    assert!(user.release_expired_lockout(), "expiry has passed");
    assert!(user.can_login());
    Ok(())
}
