//! Anonymous registration throttling: `/auth/register` is unauthenticated and
//! costs two lookups plus an Argon2 hash per call, so it gets the same per-IP
//! limit as login — in its own bucket, so the two do not throttle each other.

use super::{attempt_login_from_ip, setup_app};
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::test;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user};
use serial_test::serial;
use std::net::SocketAddr;
use std::sync::Arc;

fn register_body(username: &str) -> serde_json::Value {
    serde_json::json!({
        "username": username,
        "email": format!("{username}@example.com"),
        "password": "averysecurepassword",
        "first_name": "Reg",
        "last_name": "Ister",
    })
}

async fn attempt_register<S>(app: &S, ip: SocketAddr, username: &str) -> StatusCode
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/register")
        .peer_addr(ip)
        .set_json(register_body(username))
        .to_request();
    test::call_service(app, req).await.status()
}

async fn attempt_register_as<S>(app: &S, ip: SocketAddr, token: &str, username: &str) -> StatusCode
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/register")
        .peer_addr(ip)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(register_body(username))
        .to_request();
    test::call_service(app, req).await.status()
}

/// Log in as a freshly seeded admin and return the access token.
async fn admin_token<S>(app: &S, pool: &r_data_core_test_support::TestDatabase) -> String
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let user_uuid = create_test_admin_user(pool).await.expect("seed admin");
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let username = repo
        .find_by_uuid(&user_uuid)
        .await
        .expect("lookup")
        .expect("user exists")
        .username;

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/auth/login")
        .set_json(serde_json::json!({ "username": username, "password": "adminadmin" }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body["data"]["access_token"]
        .as_str()
        .expect("login returns an access token")
        .to_string()
}

/// Requests pass under the limit and are refused with 429 once it is reached.
#[tokio::test]
#[serial]
async fn test_anonymous_registration_is_throttled() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.30:5000".parse().unwrap();

    for attempt in 1_u8..=10 {
        let status = attempt_register(&app, ip, &format!("reg_user_{attempt}")).await;
        assert_ne!(
            status,
            StatusCode::TOO_MANY_REQUESTS,
            "attempt {attempt} is under the limit"
        );
    }

    assert_eq!(
        attempt_register(&app, ip, "reg_user_over").await,
        StatusCode::TOO_MANY_REQUESTS,
        "attempt 11 exceeds the limit → 429"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// A throttled IP on one endpoint must not throttle the other: the buckets are
/// separate, so registration probes cannot lock a shared address out of login.
#[tokio::test]
#[serial]
async fn test_register_and_login_have_separate_buckets() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.31:5000".parse().unwrap();

    for attempt in 1_u8..=11 {
        let _ = attempt_register(&app, ip, &format!("bucket_user_{attempt}")).await;
    }
    assert_eq!(
        attempt_register(&app, ip, "bucket_user_over").await,
        StatusCode::TOO_MANY_REQUESTS,
        "registration is throttled for this IP"
    );

    assert_eq!(
        attempt_login_from_ip(&app, ip, "ghost_user", "wrong_password").await,
        StatusCode::UNAUTHORIZED,
        "login from the same IP is unaffected"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// An authenticated operator creating users must not be throttled — the limit
/// exists to protect the anonymous path.
#[tokio::test]
#[serial]
async fn test_authenticated_registration_is_not_throttled() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.32:5000".parse().unwrap();
    let token = admin_token(&app, &pool).await;

    for attempt in 1_u8..=15 {
        let status = attempt_register_as(&app, ip, &token, &format!("staff_{attempt}")).await;
        assert_ne!(
            status,
            StatusCode::TOO_MANY_REQUESTS,
            "authenticated request {attempt} must not be throttled"
        );
    }

    clear_test_db(&pool).await?;
    Ok(())
}

/// The anonymous counter is untouched by authenticated traffic, so an operator
/// working through the API cannot lock the public endpoint for their own IP.
#[tokio::test]
#[serial]
async fn test_authenticated_traffic_does_not_fill_the_anonymous_bucket(
) -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let ip: SocketAddr = "203.0.113.33:5000".parse().unwrap();
    let token = admin_token(&app, &pool).await;

    for attempt in 1_u8..=15 {
        let _ = attempt_register_as(&app, ip, &token, &format!("staff_b_{attempt}")).await;
    }

    assert_ne!(
        attempt_register(&app, ip, "anon_after_staff").await,
        StatusCode::TOO_MANY_REQUESTS,
        "anonymous bucket is still empty"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
