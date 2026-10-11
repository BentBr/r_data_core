#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! That an SSO session survives its first access-token expiry.
//!
//! An SSO account stores no roles, so a refresh that read `user_roles` would
//! hand back a token with no permissions. The refresh token carries the
//! mapped roles instead.

use actix_web::{test, App};
use base64::Engine as _;
use r_data_core_api::admin::auth::routes::helpers::{issue_session_with_roles, SessionOrigin};
use r_data_core_api::{configure_app, ApiStateTrait};
use r_data_core_core::refresh_token::RefreshToken;
use r_data_core_persistence::{
    AdminUserRepository, AdminUserRepositoryTrait, RefreshTokenRepository,
    RefreshTokenRepositoryTrait,
};
use std::sync::Arc;
use uuid::Uuid;

use super::security_tests::federated_account;
use crate::api::workflows::common::build_app_state;

async fn refresh(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    refresh_token: &str,
) -> actix_web::dev::ServiceResponse {
    test::call_service(
        app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/refresh")
            .set_json(serde_json::json!({ "refresh_token": refresh_token }))
            .to_request(),
    )
    .await
}

#[actix_web::test]
async fn a_refreshed_sso_session_keeps_the_mapped_roles() {
    let (data, pool, _token, _key) = build_app_state().await.expect("app state");
    let federated = federated_account(&pool, "federated-refresh").await;

    let role_uuid: Uuid = sqlx::query_scalar(
        "INSERT INTO roles (name, permissions, created_by, published) \
         VALUES ('sso-refresh-role', $1::jsonb, $2, true) RETURNING uuid",
    )
    .bind(serde_json::json!([{
        "resource_type": "Workflows",
        "permission_type": "Read",
        "access_level": "All",
        "resource_uuids": [],
        "constraints": null,
    }]))
    .bind(Uuid::nil())
    .fetch_one(&pool.pool)
    .await
    .expect("a role");
    let role = data
        .role_service()
        .get_role(role_uuid)
        .await
        .expect("load")
        .expect("the role");

    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let mut user = repo.find_by_uuid(&federated).await.unwrap().unwrap();
    let session = issue_session_with_roles(&mut user, &[role], SessionOrigin::Sso, &repo, &data)
        .await
        .expect("a session");

    let app = test::init_service(App::new().app_data(data.clone()).configure(configure_app)).await;
    let response = refresh(&app, &session.tokens.refresh_token).await;
    assert!(response.status().is_success(), "got {}", response.status());

    let body: serde_json::Value = test::read_body_json(response).await;
    let access = body["data"]["access_token"].as_str().expect("access token");
    let payload = access.split('.').nth(1).expect("a payload segment");
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("base64");
    let claims: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
    let permissions: Vec<&str> = claims["permissions"]
        .as_array()
        .expect("permissions")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(permissions, vec!["workflows:read"]);

    // The snapshot must survive another hop, and nothing was stored.
    let next = body["data"]["refresh_token"]
        .as_str()
        .expect("refresh token");
    assert!(refresh(&app, next).await.status().is_success());
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM user_roles WHERE user_uuid = $1")
        .bind(federated)
        .fetch_one(&pool.pool)
        .await
        .expect("count");
    assert_eq!(stored, 0, "roles must not be persisted for an SSO account");
}

#[actix_web::test]
async fn an_sso_refresh_token_without_a_role_snapshot_is_refused() {
    let (data, pool, _token, _key) = build_app_state().await.expect("app state");
    let federated = federated_account(&pool, "federated-refresh-bare").await;

    let raw = RefreshToken::generate_token();
    RefreshTokenRepository::new(pool.pool.clone())
        .create(
            federated,
            RefreshToken::hash_token(&raw).expect("hash"),
            time::OffsetDateTime::now_utc() + time::Duration::hours(1),
            Some(serde_json::json!({ "user_agent": "login" })),
        )
        .await
        .expect("store");

    let app = test::init_service(App::new().app_data(data.clone()).configure(configure_app)).await;
    let response = refresh(&app, &raw).await;
    assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

/// Store a refresh row for an SSO session that signed in `ago`.
async fn sso_row_signed_in(
    pool: &r_data_core_test_support::TestDatabase,
    user: Uuid,
    ago: time::Duration,
) -> String {
    let raw = RefreshToken::generate_token();
    RefreshTokenRepository::new(pool.pool.clone())
        .create(
            user,
            RefreshToken::hash_token(&raw).expect("hash"),
            // The row itself is fresh: only the chain is old.
            time::OffsetDateTime::now_utc() + time::Duration::days(30),
            Some(serde_json::json!({
                "user_agent": "login",
                "login_time": time::OffsetDateTime::now_utc() - ago,
                "sso": true,
                "sso_role_uuids": [],
            })),
        )
        .await
        .expect("store");
    raw
}

#[actix_web::test]
async fn an_sso_chain_older_than_thirty_days_from_sign_in_is_refused() {
    // Every refresh carries the snapshot forward and gets a fresh 30-day
    // row, so without an absolute bound a chain refreshed monthly would
    // keep its sign-in roles forever.
    let (data, pool, _token, _key) = build_app_state().await.expect("app state");
    let federated = federated_account(&pool, "federated-refresh-old").await;
    let raw = sso_row_signed_in(&pool, federated, time::Duration::days(31)).await;

    let app = test::init_service(App::new().app_data(data.clone()).configure(configure_app)).await;
    let response = refresh(&app, &raw).await;
    assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn an_sso_chain_within_thirty_days_from_sign_in_still_refreshes() {
    let (data, pool, _token, _key) = build_app_state().await.expect("app state");
    let federated = federated_account(&pool, "federated-refresh-recent").await;
    let raw = sso_row_signed_in(&pool, federated, time::Duration::days(29)).await;

    let app = test::init_service(App::new().app_data(data.clone()).configure(configure_app)).await;
    let response = refresh(&app, &raw).await;
    assert!(response.status().is_success(), "got {}", response.status());
}
