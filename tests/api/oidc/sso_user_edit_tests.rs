#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! That an admin can still edit and deactivate an SSO account.
//!
//! The edit dialog always sends `role_uuids`. Refusing any list at all for a
//! federated account made every edit fail — deactivation included — so only a
//! non-empty list is refused; an empty one clears stored roles.

use actix_web::{http::StatusCode, test};
use uuid::Uuid;

use super::security_tests::federated_account;
use crate::api::users::common::{get_auth_token, setup_test_app};

async fn role(pool: &r_data_core_test_support::TestDatabase, name: &str, by: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO roles (name, created_by, published) VALUES ($1, $2, true) RETURNING uuid",
    )
    .bind(name)
    .bind(by)
    .fetch_one(&pool.pool)
    .await
    .expect("a role")
}

async fn stored_roles(pool: &r_data_core_test_support::TestDatabase, user: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM user_roles WHERE user_uuid = $1")
        .bind(user)
        .fetch_one(&pool.pool)
        .await
        .expect("count")
}

#[actix_web::test]
async fn an_sso_user_can_be_deactivated_with_an_empty_role_list() {
    let (app, pool, admin_uuid) = setup_test_app().await.expect("test app");
    let federated = federated_account(&pool, "federated-deactivate").await;
    let token = get_auth_token(&app, &pool).await;

    // A stored role left over from before claims became the authority. The
    // empty list must clear it, removing the second authority.
    let leftover = role(&pool, "sso-leftover", admin_uuid).await;
    sqlx::query("INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2)")
        .bind(federated)
        .bind(leftover)
        .execute(&pool.pool)
        .await
        .expect("seed a stored role");

    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/admin/api/v1/users/{federated}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "is_active": false, "role_uuids": [] }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let active: bool = sqlx::query_scalar("SELECT is_active FROM admin_users WHERE uuid = $1")
        .bind(federated)
        .fetch_one(&pool.pool)
        .await
        .expect("the account");
    assert!(!active, "the account must be deactivated");
    assert_eq!(
        stored_roles(&pool, federated).await,
        0,
        "stored roles cleared"
    );
}

#[actix_web::test]
async fn a_non_empty_role_list_is_still_refused_for_an_sso_user() {
    let (app, pool, admin_uuid) = setup_test_app().await.expect("test app");
    let federated = federated_account(&pool, "federated-nonempty").await;
    let token = get_auth_token(&app, &pool).await;
    let role_uuid = role(&pool, "sso-nonempty", admin_uuid).await;

    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/admin/api/v1/users/{federated}"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!({ "is_active": false, "role_uuids": [role_uuid] }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Refused before anything else was applied.
    let active: bool = sqlx::query_scalar("SELECT is_active FROM admin_users WHERE uuid = $1")
        .bind(federated)
        .fetch_one(&pool.pool)
        .await
        .expect("the account");
    assert!(active, "a refused update must not half-apply");
}

#[actix_web::test]
async fn the_roles_endpoint_accepts_an_empty_list_for_an_sso_user() {
    let (app, pool, admin_uuid) = setup_test_app().await.expect("test app");
    let federated = federated_account(&pool, "federated-roles-clear").await;
    let token = get_auth_token(&app, &pool).await;
    let leftover = role(&pool, "sso-leftover-2", admin_uuid).await;
    sqlx::query("INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2)")
        .bind(federated)
        .bind(leftover)
        .execute(&pool.pool)
        .await
        .expect("seed a stored role");

    let response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/admin/api/v1/users/{federated}/roles"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(serde_json::json!([]))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(stored_roles(&pool, federated).await, 0);
}
