#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! That an SSO-provisioned account has no password path back in.
//!
//! The service-level guards on forgot-password and reset-password are tested
//! with fakes in `crates/services`. These cover the paths that only exist as
//! HTTP handlers — login, registration, and the password field on the admin
//! user form — because a guard inside a handler is only as good as the
//! handler's wiring, and wiring is what an integration test checks.
//!
//! The account is federated by writing the flag directly rather than by
//! signing in through a provider. That keeps these tests about the guards
//! rather than about provisioning, which has its own tests.

use actix_web::{http::StatusCode, test};
use uuid::Uuid;

use crate::api::users::common::{get_auth_token, setup_test_app};

/// Argon2 hash of "adminadmin", as used by the test admin.
const KNOWN_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4";

/// A local account with a known password, marked as federated.
///
/// The password is the same one the test admin uses, and it is real and
/// correct — that is the point. A guard that only works because the password
/// happens not to match is not a guard.
///
/// The columns mirror `create_test_admin_user` rather than being written out
/// afresh: everything else about `admin_users` is defaulted by the schema, and
/// guessing at column names here is how these tests would rot.
async fn federated_account(pool: &r_data_core_test_support::TestDatabase, username: &str) -> Uuid {
    let uuid: Uuid = sqlx::query_scalar(
        "INSERT INTO admin_users \
         (username, email, password_hash, first_name, last_name, is_active, \
          created_at, updated_at, created_by, super_admin, is_sso_provisioned) \
         VALUES ($1, $2, $3, 'Fed', 'Erated', true, NOW(), NOW(), $4, false, true) \
         RETURNING uuid",
    )
    .bind(username)
    .bind(format!("{username}@example.com"))
    .bind(KNOWN_HASH)
    .bind(Uuid::nil())
    .fetch_one(&pool.pool)
    .await
    .expect("insert a federated account");

    uuid
}

#[actix_web::test]
async fn a_provisioned_user_cannot_log_in_with_a_password() {
    let (app, pool, _) = setup_test_app().await.expect("test app");
    federated_account(&pool, "federated-login").await;

    // The correct password. If this ever succeeds, the federated account has
    // become a local one that the identity provider no longer gates.
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": "federated-login",
                "password": "adminadmin"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "an SSO account must not accept a password, however correct"
    );
}

#[actix_web::test]
async fn a_provisioned_user_cannot_request_a_password_reset() {
    let (app, pool, _) = setup_test_app().await.expect("test app");
    federated_account(&pool, "federated-forgot").await;

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/forgot-password")
            .set_json(serde_json::json!({ "email": "federated-forgot@example.com" }))
            .to_request(),
    )
    .await;

    // The endpoint answers the same way for every address, by design — saying
    // otherwise would reveal which accounts exist. What must not happen is a
    // token being issued, which is asserted at the service level with fakes.
    assert_eq!(response.status(), StatusCode::OK);

    let issued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM password_reset_tokens t \
         JOIN admin_users u ON u.uuid = t.user_id \
         WHERE u.username = 'federated-forgot'",
    )
    .fetch_one(&pool.pool)
    .await
    .unwrap_or(0);

    assert_eq!(
        issued, 0,
        "forgot-password must not become a way to set a password on a federated account"
    );
}

#[actix_web::test]
async fn admin_user_update_cannot_set_a_password_on_a_provisioned_user() {
    // Without this the whole feature is bypassed by one PUT from any admin.
    let (app, pool, _) = setup_test_app().await.expect("test app");
    let uuid = federated_account(&pool, "federated-update").await;
    let token = get_auth_token(&app, &pool).await;

    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/admin/api/v1/users/{uuid}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "password": "a-brand-new-password" }))
            .to_request(),
    )
    .await;

    assert!(
        response.status().is_client_error(),
        "setting a password on a federated account must be refused, got {}",
        response.status()
    );

    // And refused rather than silently ignored: the stored hash is unchanged,
    // so nothing was half-applied.
    let after: String = sqlx::query_scalar("SELECT password_hash FROM admin_users WHERE uuid = $1")
        .bind(uuid)
        .fetch_one(&pool.pool)
        .await
        .expect("the account should still exist");

    let login = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": "federated-update",
                "password": "a-brand-new-password"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(
        after, KNOWN_HASH,
        "the refusal must leave the stored hash untouched, not half-apply the change"
    );
    assert_eq!(
        login.status(),
        StatusCode::UNAUTHORIZED,
        "the rejected password must not have been stored anyway"
    );
}

#[actix_web::test]
async fn register_cannot_create_a_local_account_shadowing_an_sso_identity() {
    // A shadowing account would give the same person two identities, one of
    // them outside the provider's control.
    let (app, pool, _) = setup_test_app().await.expect("test app");
    federated_account(&pool, "federated-shadow").await;

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/register")
            .set_json(serde_json::json!({
                "username": "federated-shadow",
                "email": "federated-shadow@example.com",
                "password": "a-shadow-password",
                "first_name": "Sh",
                "last_name": "Adow"
            }))
            .to_request(),
    )
    .await;

    // Registration answers generically whether or not the name is taken, so
    // the status says nothing. What matters is that no second row appeared.
    let _ = response.status();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM admin_users WHERE username = 'federated-shadow'")
            .fetch_one(&pool.pool)
            .await
            .unwrap_or(0);

    assert_eq!(count, 1, "registration must not shadow a federated account");

    let login = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({
                "username": "federated-shadow",
                "password": "a-shadow-password"
            }))
            .to_request(),
    )
    .await;

    assert_eq!(
        login.status(),
        StatusCode::UNAUTHORIZED,
        "and the attempted password must not work"
    );
}

#[actix_web::test]
async fn a_local_account_is_unaffected_by_any_of_this() {
    // The guards must be narrow. If they are not, password login is broken
    // for everyone — a far larger outage than the hole they close.
    let (app, pool, _) = setup_test_app().await.expect("test app");

    let username: String = sqlx::query_scalar(
        "SELECT username FROM admin_users WHERE super_admin = true ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool.pool)
    .await
    .expect("the test admin should exist");

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(serde_json::json!({ "username": username, "password": "adminadmin" }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}
