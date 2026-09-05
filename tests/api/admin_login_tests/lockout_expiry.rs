//! Expiring lockouts: a lock set by failed logins lifts itself once
//! `locked_until` has passed, while an operator lock (no expiry) does not.

use super::{attempt_login, setup_app};
use actix_web::http::StatusCode;
use r_data_core_core::admin_user::UserStatus;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user};
use serial_test::serial;
use std::sync::Arc;
use time::OffsetDateTime;

/// Locking through failed logins records an expiry, so the lock is temporary.
#[tokio::test]
#[serial]
async fn test_automatic_lockout_records_an_expiry() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    for _ in 1_u8..=5 {
        attempt_login(&app, &username, "wrong_password").await;
    }

    let locked = repo.find_by_uuid(&user_uuid).await?.unwrap();
    assert_eq!(locked.status, UserStatus::Locked);
    assert!(
        locked
            .locked_until
            .is_some_and(|t| t > OffsetDateTime::now_utc()),
        "automatic lockout must carry a future expiry"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Once the expiry has passed the next correct-password login succeeds and the
/// account is back to Active with a clean counter — no operator needed.
#[tokio::test]
#[serial]
async fn test_expired_lockout_lets_the_user_back_in() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    sqlx::query(
        "UPDATE admin_users SET status = 'locked', failed_login_attempts = 5, \
         locked_until = NOW() - INTERVAL '1 minute' WHERE uuid = $1",
    )
    .bind(user_uuid)
    .execute(&pool.pool)
    .await
    .expect("failed to pre-lock account");

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::OK, "expired lockout → login succeeds");

    let after = repo.find_by_uuid(&user_uuid).await?.unwrap();
    assert_eq!(after.status, UserStatus::Active);
    assert_eq!(after.failed_login_attempts, 0);
    assert!(after.locked_until.is_none());

    clear_test_db(&pool).await?;
    Ok(())
}

/// A lockout that has not expired yet still blocks, even with the right password.
#[tokio::test]
#[serial]
async fn test_unexpired_lockout_still_blocks() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    sqlx::query(
        "UPDATE admin_users SET status = 'locked', failed_login_attempts = 5, \
         locked_until = NOW() + INTERVAL '1 hour' WHERE uuid = $1",
    )
    .bind(user_uuid)
    .execute(&pool.pool)
    .await
    .expect("failed to pre-lock account");

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "lock still in force → 403");

    clear_test_db(&pool).await?;
    Ok(())
}

/// An operator lock has no expiry and must never lift itself.
#[tokio::test]
#[serial]
async fn test_operator_lock_never_expires() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    // `locked_until` NULL is what the user_actions CLI writes.
    sqlx::query(
        "UPDATE admin_users SET status = 'locked', failed_login_attempts = 5, \
         locked_until = NULL WHERE uuid = $1",
    )
    .bind(user_uuid)
    .execute(&pool.pool)
    .await
    .expect("failed to pre-lock account");

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "operator lock → 403");

    assert_eq!(
        repo.find_by_uuid(&user_uuid).await?.unwrap().status,
        UserStatus::Locked,
        "operator lock must survive a login attempt"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
