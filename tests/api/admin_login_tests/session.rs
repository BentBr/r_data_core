//! Session lifecycle over the auth endpoints: refresh, logout, revoke-all, and
//! the permissions probe the admin shell calls on boot.
//!
//! These sit next to the lockout and rate-limit work because they are the rest
//! of the unauthenticated-ish auth surface: a refresh token is a bearer
//! credential, so rotating and revoking it has to actually take effect.

use super::setup_app;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::test;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user, TestDatabase};
use serial_test::serial;
use std::sync::Arc;

const LOGIN: &str = "/admin/api/v1/auth/login";
const REFRESH: &str = "/admin/api/v1/auth/refresh";
const LOGOUT: &str = "/admin/api/v1/auth/logout";
const REVOKE_ALL: &str = "/admin/api/v1/auth/revoke-all";
const PERMISSIONS: &str = "/admin/api/v1/auth/permissions";

struct Session {
    access: String,
    refresh: String,
    username: String,
}

/// Log in again as an existing account, for tests that need two live sessions.
async fn login_again<S>(app: &S, username: &str) -> Session
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri(LOGIN)
        .set_json(serde_json::json!({ "username": username, "password": "adminadmin" }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;

    Session {
        access: body["data"]["access_token"]
            .as_str()
            .expect("access token")
            .to_string(),
        refresh: body["data"]["refresh_token"]
            .as_str()
            .expect("refresh token")
            .to_string(),
        username: username.to_string(),
    }
}

/// Seed an admin and log in, returning both halves of the token pair.
async fn login<S>(app: &S, pool: &TestDatabase) -> Session
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

    login_again(app, &username).await
}

async fn post_refresh<S>(app: &S, token: &str) -> ServiceResponse
where
    S: Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let req = test::TestRequest::post()
        .uri(REFRESH)
        .set_json(serde_json::json!({ "refresh_token": token }))
        .to_request();
    test::call_service(app, req).await
}

#[tokio::test]
#[serial]
async fn test_refresh_issues_a_new_token_pair() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let session = login(&app, &pool).await;

    let resp = post_refresh(&app, &session.refresh).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let new_refresh = body["data"]["refresh_token"]
        .as_str()
        .expect("refresh token");
    assert!(body["data"]["access_token"].is_string());
    assert_ne!(
        new_refresh, session.refresh,
        "the refresh token must rotate, not be handed back unchanged"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Rotation is only worth anything if the old token stops working.
#[tokio::test]
#[serial]
async fn test_the_old_refresh_token_stops_working_after_rotation(
) -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let session = login(&app, &pool).await;

    let first = post_refresh(&app, &session.refresh).await;
    assert_eq!(first.status(), StatusCode::OK);

    let reused = post_refresh(&app, &session.refresh).await;
    assert_eq!(
        reused.status(),
        StatusCode::UNAUTHORIZED,
        "a rotated refresh token must not be reusable"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_refresh_rejects_a_made_up_token() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let _session = login(&app, &pool).await;

    let resp = post_refresh(&app, "not-a-real-refresh-token").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_logout_revokes_the_refresh_token() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let session = login(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(LOGOUT)
        .insert_header(("Authorization", format!("Bearer {}", session.access)))
        .set_json(serde_json::json!({ "refresh_token": session.refresh }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "logout failed: {}",
        resp.status()
    );

    let after = post_refresh(&app, &session.refresh).await;
    assert_eq!(
        after.status(),
        StatusCode::UNAUTHORIZED,
        "a logged-out session must not refresh"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Revoke-all is the "I think I have been compromised" button, so every
/// outstanding refresh token has to die, not just the caller's own.
#[tokio::test]
#[serial]
async fn test_revoke_all_kills_every_outstanding_session() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let first = login(&app, &pool).await;
    // A second, independent session for the same account.
    let second = login_again(&app, &first.username).await;

    let req = test::TestRequest::post()
        .uri(REVOKE_ALL)
        .insert_header(("Authorization", format!("Bearer {}", first.access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "revoke-all failed: {}",
        resp.status()
    );

    assert_eq!(
        post_refresh(&app, &first.refresh).await.status(),
        StatusCode::UNAUTHORIZED,
        "the caller's own session must be revoked"
    );
    assert_eq!(
        post_refresh(&app, &second.refresh).await.status(),
        StatusCode::UNAUTHORIZED,
        "every other session for the account must be revoked too"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_permissions_probe_describes_the_caller() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;
    let session = login(&app, &pool).await;

    let req = test::TestRequest::get()
        .uri(PERMISSIONS)
        .insert_header(("Authorization", format!("Bearer {}", session.access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["data"].is_object() || body["data"].is_array(),
        "permissions payload should describe the caller: {body}"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_session_endpoints_reject_anonymous_callers() -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    for req in [
        test::TestRequest::get().uri(PERMISSIONS).to_request(),
        test::TestRequest::post().uri(REVOKE_ALL).to_request(),
    ] {
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    clear_test_db(&pool).await?;
    Ok(())
}
