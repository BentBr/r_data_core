#![allow(clippy::expect_used, clippy::unwrap_used)]

//! A refreshed SSO session after the mapped role changed underneath it.
//!
//! The refresh chain carries the role uuids from sign-in, and the role is
//! re-read on refresh: a deleted role must grant nothing, and a role that
//! lost a permission must not keep handing it out.
//!
//! Serialised: resource-heavy stacks (full server + MCP + migrated schema).

use serial_test::serial;

use super::callback_e2e_tests::{callback, permissions_in, start};
use super::refresh_authority_tests::{claims_of, percent_decode, permissions_of};
use crate::mcp::oauth_harness::OauthStack;

/// Sign in through the browser flow; returns the refresh token.
async fn sign_in(stack: &OauthStack, subject: &str, group: &str) -> (String, Vec<String>) {
    let (state, nonce, cookie) = start(stack).await;
    let location = callback(
        stack,
        &state,
        &cookie,
        &stack.mint_id_token(subject, &[group], &nonce),
    )
    .await;
    assert!(!location.contains("sso_error"), "{location}");
    let refresh = percent_decode(
        location
            .split('#')
            .nth(1)
            .and_then(|f| f.split('&').find_map(|p| p.strip_prefix("refresh_token=")))
            .expect("a refresh token"),
    );
    (refresh, permissions_in(&location))
}

/// Refresh; returns the permissions on the new access token.
async fn refresh(stack: &OauthStack, refresh_token: &str) -> Vec<String> {
    let response = reqwest::Client::new()
        .post(format!("{}/admin/api/v1/auth/refresh", stack.rdc_url))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .expect("a response");
    assert_eq!(response.status(), 200, "the refresh must still succeed");
    let body: serde_json::Value = response.json().await.expect("json");
    permissions_of(&claims_of(
        body["data"]["access_token"].as_str().expect("access"),
    ))
}

fn two_permissions() -> serde_json::Value {
    serde_json::json!([
        {
            "resource_type": "Workflows", "permission_type": "Read",
            "access_level": "All", "resource_uuids": [], "constraints": null,
        },
        {
            "resource_type": "Workflows", "permission_type": "Create",
            "access_level": "All", "resource_uuids": [], "constraints": null,
        },
    ])
}

#[tokio::test]
#[serial]
async fn refresh_after_the_mapped_role_is_deleted_carries_no_permissions() {
    let stack = OauthStack::start("rdc-ops:sso-doomed").await;
    stack
        .grant_permission("sso-doomed", "Workflows", "Read")
        .await;

    let (refresh_token, at_sign_in) = sign_in(&stack, "doomed", "rdc-ops").await;
    assert_eq!(at_sign_in, vec!["workflows:read".to_string()]);

    sqlx::query("DELETE FROM roles WHERE name = 'sso-doomed'")
        .execute(stack.pool())
        .await
        .expect("delete the role");

    assert!(
        refresh(&stack, &refresh_token).await.is_empty(),
        "a deleted role must grant nothing on refresh"
    );
}

#[tokio::test]
#[serial]
async fn refresh_after_a_permission_is_removed_drops_only_that_permission() {
    let stack = OauthStack::start("rdc-ops:sso-shrink").await;
    sqlx::query("UPDATE roles SET permissions = $1::jsonb WHERE name = 'sso-shrink'")
        .bind(two_permissions())
        .execute(stack.pool())
        .await
        .expect("grant two permissions");

    let (refresh_token, mut at_sign_in) = sign_in(&stack, "shrinker", "rdc-ops").await;
    at_sign_in.sort();
    assert_eq!(
        at_sign_in,
        vec!["workflows:create".to_string(), "workflows:read".to_string()]
    );

    // Remove "create", keep "read".
    stack
        .grant_permission("sso-shrink", "Workflows", "Read")
        .await;

    assert_eq!(
        refresh(&stack, &refresh_token).await,
        vec!["workflows:read".to_string()],
        "the removed permission must go, the others stay"
    );
}
