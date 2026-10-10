#![allow(clippy::expect_used, clippy::unwrap_used)]

//! End-to-end: a real identity, through a real exchange, against a real
//! `RDataCore`.
//!
//! The crate's own tests prove the MCP server behaves correctly against the
//! shapes it *expects*. These prove those shapes are the ones `RDataCore`
//! actually produces, and — the part that matters most — that the identity it
//! ends up acting as is the human who presented the token, not a service
//! account.
//!
//! Serialised, because the harness sets `RDC_OIDC_*` in the process
//! environment: `RDataCore` reads those at startup, and two of these running
//! at once would configure each other's servers.

use base64::Engine as _;
use serial_test::serial;

use super::oauth_harness::OauthStack;

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("a client")
}

/// One JSON-RPC call against the MCP endpoint.
async fn mcp_call(stack: &OauthStack, token: &str, body: serde_json::Value) -> reqwest::Response {
    client()
        .post(format!("{}/mcp", stack.mcp_url))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .expect("a response")
}

#[tokio::test]
#[serial]
async fn a_valid_token_reaches_the_tool_surface() {
    let stack = OauthStack::start("rdc-admins:admin").await;
    let token = stack.mint_token("alice", &["rdc-admins"]);

    let response = mcp_call(
        &stack,
        &token,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "e2e", "version": "0" }
            }
        }),
    )
    .await;

    assert!(
        response.status().is_success(),
        "a token this instance trusts must be accepted, got {}",
        response.status()
    );
}

#[tokio::test]
#[serial]
async fn a_token_from_an_untrusted_issuer_is_refused() {
    let stack = OauthStack::start("rdc-admins:admin").await;

    // Signed by nothing this instance trusts.
    let response = mcp_call(&stack, "not.a.real.token", serde_json::json!({})).await;

    assert_eq!(response.status(), 401);
    assert!(
        response.headers().get("WWW-Authenticate").is_some(),
        "even a refusal must tell a client where to authenticate"
    );
}

#[tokio::test]
#[serial]
async fn an_unmapped_identity_cannot_obtain_a_local_token() {
    // Default-deny reaches all the way through: the token is genuine and
    // validates, but the person maps to no role, so the exchange refuses and
    // the MCP server has nothing to act with.
    let stack = OauthStack::start("rdc-admins:admin").await;
    let token = stack.mint_token("nobody", &["a-group-nobody-mapped"]);

    let response = client()
        .post(format!("{}/admin/api/v1/auth/oidc/exchange", stack.rdc_url))
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .expect("a response");

    assert!(
        response.status().is_client_error(),
        "an unmapped identity must not be issued a local token, got {}",
        response.status()
    );
}

#[tokio::test]
#[serial]
async fn the_exchange_returns_a_token_for_the_real_person() {
    // If this returned a service identity the audit trail would be worthless:
    // every change made through MCP would be attributed to the same account.
    let stack = OauthStack::start("rdc-admins:admin").await;
    let token = stack.mint_token("alice", &["rdc-admins"]);

    let response = client()
        .post(format!("{}/admin/api/v1/auth/oidc/exchange", stack.rdc_url))
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .expect("a response");

    assert!(
        response.status().is_success(),
        "expected an exchange, got {}",
        response.status()
    );

    let body: serde_json::Value = response.json().await.expect("json");
    let access = body["data"]["access_token"]
        .as_str()
        .expect("an access token");

    assert_ne!(
        access, token,
        "the local token must not be the presented one"
    );

    // The local token is a plain RDataCore JWT; read its subject back.
    let payload = access.split('.').nth(1).expect("a payload segment");
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("base64");
    let claims: serde_json::Value = serde_json::from_slice(&decoded).expect("json");

    assert_eq!(
        claims["email"], "alice@example.com",
        "the local token must name the human, not a service account"
    );
    assert_eq!(claims["iss"], "r_data_core_admin");
}

#[tokio::test]
#[serial]
async fn a_readonly_caller_is_refused_by_rdatacore_not_merely_hidden_from() {
    // Tool hiding is ergonomics. This proves enforcement still happens
    // server-side, so a client calling a tool it was never offered gains
    // nothing.
    let stack = OauthStack::start("rdc-readers:reader").await;
    let token = stack.mint_token("reader", &["rdc-readers"]);

    let exchanged = client()
        .post(format!("{}/admin/api/v1/auth/oidc/exchange", stack.rdc_url))
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .expect("a response");

    // Either the identity maps to no usable role and is refused outright, or
    // it is admitted and then refused at the write endpoint. Both are correct;
    // what must not happen is a successful write.
    if !exchanged.status().is_success() {
        return;
    }

    let body: serde_json::Value = exchanged.json().await.expect("json");
    let access = body["data"]["access_token"]
        .as_str()
        .expect("an access token");

    let created = client()
        .post(format!("{}/admin/api/v1/workflows", stack.rdc_url))
        .header("Authorization", format!("Bearer {access}"))
        .json(&serde_json::json!({
            "name": "should-not-exist",
            "description": "a read-only caller must not create this",
        }))
        .send()
        .await
        .expect("a response");

    assert!(
        !created.status().is_success(),
        "a read-only caller must be refused server-side, got {}",
        created.status()
    );
}
