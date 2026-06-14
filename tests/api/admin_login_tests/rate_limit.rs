//! Per-IP login rate limiting: requests pass under the limit, are blocked with
//! 429 once the limit is reached, and the counter resets after a success.
//!
//! `MAX_ATTEMPTS = 10` failed attempts per IP within the window. Every failed
//! attempt counts — bad password, unknown user, or locked/inactive account — so
//! username-enumeration from one IP is throttled too.

use super::{attempt_login_from_ip, setup_app};
use actix_web::http::StatusCode;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user};
use serial_test::serial;
use std::net::SocketAddr;
use std::sync::Arc;

/// Red path: ten failed attempts from one IP, then the eleventh is blocked with
/// 429 — and the under-limit attempts are NOT blocked.
#[tokio::test]
#[serial]
async fn test_rate_limit_blocks_after_max_attempts() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.10:5000".parse().unwrap();

    // Probe a non-existent user so account-level lockout never interferes; each
    // failure still increments the per-IP counter.
    for attempt in 1_u8..=10 {
        let status = attempt_login_from_ip(&app, ip, "ghost_user", "wrong_password").await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "attempt {attempt} is under the limit → 401, not throttled"
        );
    }

    // Eleventh attempt is over the limit → 429.
    let status = attempt_login_from_ip(&app, ip, "ghost_user", "wrong_password").await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "attempt 11 exceeds MAX_ATTEMPTS → 429"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Different IPs have independent counters: a throttled IP does not affect a
/// fresh one.
#[tokio::test]
#[serial]
async fn test_rate_limit_is_per_ip() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let blocked: SocketAddr = "203.0.113.11:5000".parse().unwrap();
    let fresh: SocketAddr = "203.0.113.12:5000".parse().unwrap();

    for _ in 0..11 {
        let _ = attempt_login_from_ip(&app, blocked, "ghost_user", "wrong_password").await;
    }
    assert_eq!(
        attempt_login_from_ip(&app, blocked, "ghost_user", "wrong_password").await,
        StatusCode::TOO_MANY_REQUESTS,
        "blocked IP stays throttled"
    );

    // A different IP is unaffected → normal 401, not 429.
    assert_eq!(
        attempt_login_from_ip(&app, fresh, "ghost_user", "wrong_password").await,
        StatusCode::UNAUTHORIZED,
        "fresh IP is not throttled"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Green path: a successful login clears the IP counter, so a user who nearly
/// hit the limit is not thrown into 429 afterwards.
#[tokio::test]
#[serial]
async fn test_successful_login_clears_rate_limit() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.20:5000".parse().unwrap();

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    // Nine failures from this IP — one below the limit, so still allowed through.
    for _ in 0..9 {
        let status = attempt_login_from_ip(&app, ip, "ghost_user", "wrong_password").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // A successful login from the same IP must succeed and reset the counter.
    let status = attempt_login_from_ip(&app, ip, &username, "adminadmin").await;
    assert_eq!(status, StatusCode::OK, "valid login under the limit → 200");

    // Because the counter reset, a further failure is a normal 401, not 429.
    let status = attempt_login_from_ip(&app, ip, "ghost_user", "wrong_password").await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "counter cleared on success → next failure is 401, not 429"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
