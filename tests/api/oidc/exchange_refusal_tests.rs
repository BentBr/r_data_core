#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The machine door, closed: `/auth/oidc/exchange` must refuse a bad token or
//! a barred account, and must hand back nothing a caller could use.
//!
//! Serialised: the harness builds a full `RDataCore` and mock provider.

use serial_test::serial;

use crate::mcp::oauth_harness::OauthStack;

const ROLE_MAP: &str = "rdc-admins:admin";

/// Post `token` to the exchange; returns the status and the JSON body.
async fn exchange(stack: &OauthStack, token: &str) -> (u16, serde_json::Value) {
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("a client")
        .post(format!("{}/admin/api/v1/auth/oidc/exchange", stack.rdc_url))
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .expect("a response");
    let status = response.status().as_u16();
    let body = response.json().await.unwrap_or(serde_json::Value::Null);
    (status, body)
}

/// A refusal must not carry a credential in any form.
fn assert_no_session(body: &serde_json::Value) {
    assert!(
        body["data"]["access_token"].is_null() && body["data"]["refresh_token"].is_null(),
        "a refused exchange must not return a token: {body}"
    );
}

async fn user_count(stack: &OauthStack, email: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM admin_users WHERE email = $1")
        .bind(email)
        .fetch_one(stack.pool())
        .await
        .expect("count users")
}

/// Sign in once so the account exists, then return the address it was
/// provisioned under.
async fn signed_in_before(stack: &OauthStack, subject: &str) -> String {
    let (status, body) = exchange(stack, &stack.mint_token(subject, &["rdc-admins"])).await;
    assert_eq!(status, 200, "the first sign-in must succeed: {body}");
    format!("{subject}@example.com")
}

#[tokio::test]
#[serial]
async fn a_tampered_signature_is_refused() {
    let stack = OauthStack::start(ROLE_MAP).await;
    let mut token = stack.mint_token("tamper", &["rdc-admins"]);
    // Change a character in the middle of the signature segment: the last
    // character of an RSA signature can carry only padding bits, so changing it
    // may be rejected as malformed, or accepted by a lenient decoder.
    let sig_start = token.rfind('.').expect("a signature segment") + 1;
    let at = sig_start + (token.len() - sig_start) / 2;
    let original = token.as_bytes()[at] as char;
    token.replace_range(at..=at, if original == 'A' { "B" } else { "A" });

    let (status, body) = exchange(&stack, &token).await;

    assert_eq!(status, 401, "{body}");
    assert_no_session(&body);
    assert_eq!(user_count(&stack, "tamper@example.com").await, 0);
}

#[tokio::test]
#[serial]
async fn a_token_for_another_audience_is_refused() {
    let stack = OauthStack::start(ROLE_MAP).await;
    let mut claims = stack.claims_for("elsewhere", &["rdc-admins"]);
    claims["aud"] = serde_json::json!("someone-else");

    let (status, body) = exchange(&stack, &stack.sign(&claims)).await;

    assert_eq!(status, 401, "{body}");
    assert_no_session(&body);
    assert_eq!(user_count(&stack, "elsewhere@example.com").await, 0);
}

#[tokio::test]
#[serial]
async fn an_expired_token_is_refused() {
    let stack = OauthStack::start(ROLE_MAP).await;
    let mut claims = stack.claims_for("stale", &["rdc-admins"]);
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    claims["iat"] = serde_json::json!(now - 7200);
    claims["exp"] = serde_json::json!(now - 3600);

    let (status, body) = exchange(&stack, &stack.sign(&claims)).await;

    assert_eq!(status, 401, "{body}");
    assert_no_session(&body);
    assert_eq!(user_count(&stack, "stale@example.com").await, 0);
}

#[tokio::test]
#[serial]
async fn a_deactivated_account_is_refused_despite_a_valid_token() {
    let stack = OauthStack::start(ROLE_MAP).await;
    let email = signed_in_before(&stack, "dora").await;
    sqlx::query("UPDATE admin_users SET is_active = FALSE WHERE email = $1")
        .bind(&email)
        .execute(stack.pool())
        .await
        .expect("deactivate");

    let (status, body) = exchange(&stack, &stack.mint_token("dora", &["rdc-admins"])).await;

    assert_eq!(status, 403, "{body}");
    assert_no_session(&body);
}

#[tokio::test]
#[serial]
async fn a_locked_account_is_refused_despite_a_valid_token() {
    let stack = OauthStack::start(ROLE_MAP).await;
    let email = signed_in_before(&stack, "lock").await;
    sqlx::query("UPDATE admin_users SET locked_until = NOW() + INTERVAL '1 hour' WHERE email = $1")
        .bind(&email)
        .execute(stack.pool())
        .await
        .expect("lock");

    let (status, body) = exchange(&stack, &stack.mint_token("lock", &["rdc-admins"])).await;

    assert_eq!(status, 403, "{body}");
    assert_no_session(&body);
}
