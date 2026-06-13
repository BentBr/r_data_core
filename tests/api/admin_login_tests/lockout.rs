//! Account-lockout behaviour: lock after 5 failures, reset on success, and the
//! one-below-threshold boundary.

use super::{attempt_login, setup_app};
use actix_web::http::StatusCode;
use r_data_core_core::admin_user::UserStatus;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user};
use serial_test::serial;
use std::sync::Arc;

/// Five consecutive bad-password attempts lock the account; a subsequent
/// correct-password login is then rejected with 403.
#[tokio::test]
#[serial]
async fn test_account_locked_after_five_failures() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    for attempt in 1_u8..=5 {
        let status = attempt_login(&app, &username, "wrong_password").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {attempt} → 401");
    }

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "correct password after lockout → 403"
    );

    let locked = repo.find_by_uuid(&user_uuid).await?.unwrap();
    assert!(locked.failed_login_attempts >= 5);
    assert_eq!(locked.status, UserStatus::Locked);

    clear_test_db(&pool).await?;
    Ok(())
}

/// Boundary (green): exactly one attempt below the threshold (4 failures) must
/// still allow a correct-password login through, and reset the counter.
#[tokio::test]
#[serial]
async fn test_four_failures_then_success_passes() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    // Four failures — one below the lockout threshold of five.
    for attempt in 1_u8..=4 {
        let status = attempt_login(&app, &username, "wrong_password").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {attempt} → 401");
    }
    assert_eq!(
        repo.find_by_uuid(&user_uuid)
            .await?
            .unwrap()
            .failed_login_attempts,
        4,
        "counter should be 4, still below the threshold"
    );

    // The fifth attempt with the CORRECT password must succeed (account not locked).
    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::OK, "login one-below-threshold → 200");

    let after = repo.find_by_uuid(&user_uuid).await?.unwrap();
    assert_eq!(
        after.failed_login_attempts, 0,
        "counter reset after success"
    );
    assert_eq!(after.status, UserStatus::Active);

    clear_test_db(&pool).await?;
    Ok(())
}

/// Fewer than five failures followed by a successful login resets the counter.
#[tokio::test]
#[serial]
async fn test_successful_login_resets_failure_counter() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    for attempt in 1_u8..=3 {
        let status = attempt_login(&app, &username, "wrong_password").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {attempt} → 401");
    }
    assert_eq!(
        repo.find_by_uuid(&user_uuid)
            .await?
            .unwrap()
            .failed_login_attempts,
        3
    );

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::OK, "correct-password login → 200");

    let after = repo.find_by_uuid(&user_uuid).await?.unwrap();
    assert_eq!(after.failed_login_attempts, 0, "counter reset to 0");
    assert_eq!(after.status, UserStatus::Active);

    clear_test_db(&pool).await?;
    Ok(())
}

/// A pre-locked account (status set directly) is rejected with 403, proving the
/// `can_login()` gate fires regardless of how the lock was reached.
#[tokio::test]
#[serial]
async fn test_pre_locked_account_returns_403() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    sqlx::query(
        "UPDATE admin_users SET status = 'Locked', failed_login_attempts = 5 WHERE uuid = $1",
    )
    .bind(user_uuid)
    .execute(&pool.pool)
    .await
    .expect("failed to pre-lock account");

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "pre-locked account → 403 even with correct password"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// An inactive account (`is_active = false`) is rejected with 403.
#[tokio::test]
#[serial]
async fn test_inactive_account_returns_403() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    sqlx::query("UPDATE admin_users SET is_active = false WHERE uuid = $1")
        .bind(user_uuid)
        .execute(&pool.pool)
        .await
        .expect("failed to deactivate account");

    let status = attempt_login(&app, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "inactive account → 403");

    clear_test_db(&pool).await?;
    Ok(())
}
