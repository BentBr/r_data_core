#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! What a refreshed SSO session is allowed to do.
//!
//! An SSO session's authority is the roles its sign-in mapped to, carried on
//! the refresh chain. A refresh must not widen that: not by reading the
//! account's stored roles (an email-adopted account keeps them), and not by
//! picking up a role that became super-admin after sign-in.

use actix_web::{test, App};
use base64::Engine as _;
use r_data_core_api::admin::auth::routes::helpers::{issue_session_with_roles, SessionOrigin};
use r_data_core_api::{configure_app, ApiStateTrait};
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use super::callback_e2e_tests::{callback, permissions_in, start};
use super::security_tests::federated_account;
use crate::api::workflows::common::build_app_state;
use crate::mcp::oauth_harness::OauthStack;

/// The claims of an access token.
fn claims_of(access: &str) -> serde_json::Value {
    let payload = access.split('.').nth(1).expect("a payload segment");
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("base64");
    serde_json::from_slice(&decoded).expect("json")
}

fn permissions_of(claims: &serde_json::Value) -> Vec<String> {
    claims["permissions"]
        .as_array()
        .expect("permissions")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

/// Undo the callback's fragment encoding (`%XX` only).
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).expect("utf8");
            out.push(u8::from_str_radix(hex, 16).expect("hex"));
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("utf8")
}

fn permission(resource: &str, permission_type: &str) -> serde_json::Value {
    serde_json::json!([{
        "resource_type": resource,
        "permission_type": permission_type,
        "access_level": "All",
        "resource_uuids": [],
        "constraints": null,
    }])
}

#[actix_web::test]
async fn a_role_made_super_admin_after_sign_in_is_dropped_on_refresh() {
    let (data, pool, _token, _key) = build_app_state().await.expect("app state");
    let federated = federated_account(&pool, "federated-refresh-escalate").await;

    let role_uuid: Uuid = sqlx::query_scalar(
        "INSERT INTO roles (name, permissions, created_by, published) \
         VALUES ('sso-escalate-role', $1::jsonb, $2, true) RETURNING uuid",
    )
    .bind(permission("Workflows", "Read"))
    .bind(Uuid::nil())
    .fetch_one(&pool.pool)
    .await
    .expect("a role");
    let mut role = data
        .role_service()
        .get_role(role_uuid)
        .await
        .expect("load")
        .expect("the role");

    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let mut user = repo.find_by_uuid(&federated).await.unwrap().unwrap();
    let session = issue_session_with_roles(
        &mut user,
        std::slice::from_ref(&role),
        SessionOrigin::Sso,
        &repo,
        &data,
    )
    .await
    .expect("a session");

    // Sign-in mapping never grants super-admin; neither may a refresh.
    role.super_admin = true;
    data.role_service()
        .update_role(&role, Uuid::nil())
        .await
        .expect("flip the role to super-admin");

    let app = test::init_service(App::new().app_data(data.clone()).configure(configure_app)).await;
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/admin/api/v1/auth/refresh")
            .set_json(serde_json::json!({ "refresh_token": session.tokens.refresh_token }))
            .to_request(),
    )
    .await;
    assert!(response.status().is_success(), "got {}", response.status());

    let body: serde_json::Value = test::read_body_json(response).await;
    let claims = claims_of(body["data"]["access_token"].as_str().expect("access"));
    assert_eq!(claims["is_super_admin"], false, "refresh must not escalate");
    assert!(
        permissions_of(&claims).is_empty(),
        "the super-admin role must be dropped, not merely demoted: {claims}"
    );
}

#[tokio::test]
#[serial]
async fn an_adopted_accounts_refresh_keeps_the_mapped_roles_not_the_stored_ones() {
    // Adoption keeps `is_sso_provisioned = false`, so keying the snapshot on
    // that flag let the first refresh fall back to the stored roles.
    std::env::set_var("RDC_OIDC_LINK_BY_EMAIL", "true");
    let stack = OauthStack::start("rdc-ops:sso-editor").await;
    stack
        .grant_permission("sso-editor", "Workflows", "Read")
        .await;

    // A local account with a stored role the provider never granted.
    let local: Uuid = sqlx::query_scalar(
        "INSERT INTO admin_users \
         (username, email, password_hash, first_name, last_name, is_active, \
          created_at, updated_at, created_by, super_admin, is_sso_provisioned) \
         VALUES ('adoptee-local', 'adoptee@example.com', 'unusable', 'Ad', 'Optee', true, \
                 NOW(), NOW(), $1, false, false) RETURNING uuid",
    )
    .bind(Uuid::nil())
    .fetch_one(stack.pool())
    .await
    .expect("a local account");
    let stored_role: Uuid = sqlx::query_scalar(
        "INSERT INTO roles (name, permissions, created_by, published) \
         VALUES ('adoptee-stored', $1::jsonb, $2, true) RETURNING uuid",
    )
    .bind(permission("Workflows", "Delete"))
    .bind(Uuid::nil())
    .fetch_one(stack.pool())
    .await
    .expect("a stored role");
    sqlx::query("INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2)")
        .bind(local)
        .bind(stored_role)
        .execute(stack.pool())
        .await
        .expect("store the role");

    let (state, nonce, cookie) = start(&stack).await;
    let location = callback(
        &stack,
        &state,
        &cookie,
        &stack.mint_id_token("adoptee", &["rdc-ops"], &nonce),
    )
    .await;
    assert!(!location.contains("sso_error"), "{location}");
    assert_eq!(
        permissions_in(&location),
        vec!["workflows:read".to_string()]
    );

    let adopted: bool =
        sqlx::query_scalar("SELECT is_sso_provisioned FROM admin_users WHERE uuid = $1")
            .bind(local)
            .fetch_one(stack.pool())
            .await
            .expect("the account");
    assert!(
        !adopted,
        "precondition: adoption leaves the account local by flag"
    );

    let refresh_token = percent_decode(
        location
            .split('#')
            .nth(1)
            .and_then(|f| f.split('&').find_map(|p| p.strip_prefix("refresh_token=")))
            .expect("a refresh token"),
    );
    let response = reqwest::Client::new()
        .post(format!("{}/admin/api/v1/auth/refresh", stack.rdc_url))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .expect("a response");
    assert!(response.status().is_success(), "got {}", response.status());

    let body: serde_json::Value = response.json().await.expect("json");
    let claims = claims_of(body["data"]["access_token"].as_str().expect("access"));
    assert_eq!(
        permissions_of(&claims),
        vec!["workflows:read".to_string()],
        "the refresh must carry the mapped role, not the stored workflows:delete"
    );
}
