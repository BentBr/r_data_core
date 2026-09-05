#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions use impl Service from setup_test_app() across awaits

//! Operator unlock through `PUT /users/{uuid}`: sending `status` is the
//! supported way back from a locked account, and it clears the lockout
//! bookkeeping so the next login is not immediately blocked again.

use actix_web::{http::StatusCode, test};
use r_data_core_core::admin_user::UserStatus;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use serial_test::serial;
use std::sync::Arc;

use super::common::{get_auth_token, setup_test_app};

async fn lock_user(pool: &r_data_core_test_support::TestDatabase, uuid: uuid::Uuid) {
    sqlx::query(
        "UPDATE admin_users SET status = 'locked', failed_login_attempts = 5, \
         locked_until = NOW() + INTERVAL '1 hour' WHERE uuid = $1",
    )
    .bind(uuid)
    .execute(&pool.pool)
    .await
    .expect("failed to lock account");
}

#[serial]
#[tokio::test]
async fn test_update_status_unlocks_account() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;
    lock_user(&pool, user_uuid).await;

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "status": "active" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "active");
    assert_eq!(body["data"]["failed_login_attempts"], 0);
    assert!(body["data"]["locked_until"].is_null());

    // The unlock must be persisted, not just reflected in the response.
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let stored = repo.find_by_uuid(&user_uuid).await.unwrap().unwrap();
    assert_eq!(stored.status, UserStatus::Active);
    assert_eq!(stored.failed_login_attempts, 0);
    assert!(stored.locked_until.is_none());
    assert!(stored.can_login());
}

#[serial]
#[tokio::test]
async fn test_update_status_can_lock_an_account() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "status": "locked" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let stored = repo.find_by_uuid(&user_uuid).await.unwrap().unwrap();
    assert_eq!(stored.status, UserStatus::Locked);
    // An operator lock carries no expiry, so it does not lift itself.
    assert!(stored.locked_until.is_none());
    assert!(!stored.can_login());
}

#[serial]
#[tokio::test]
async fn test_update_without_status_leaves_lock_untouched() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;
    lock_user(&pool, user_uuid).await;

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "first_name": "Renamed" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let stored = repo.find_by_uuid(&user_uuid).await.unwrap().unwrap();
    assert_eq!(stored.status, UserStatus::Locked);
    assert_eq!(stored.failed_login_attempts, 5);
    assert!(stored.locked_until.is_some());
}

#[serial]
#[tokio::test]
async fn test_unknown_status_value_is_rejected() {
    let (app, pool, user_uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/users/{user_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "status": "unlocked_please" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
