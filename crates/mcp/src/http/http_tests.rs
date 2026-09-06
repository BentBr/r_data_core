#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What the HTTP surface must do before any tool is reached.
//!
//! These start a real listener and speak real HTTP to it, because the things
//! being asserted — a status code, a header, which paths need a token — are
//! properties of the transport rather than of any function it calls.

use std::collections::HashMap;

use super::*;

/// A configured server, bound to an ephemeral port.
struct RunningServer {
    base: String,
    shutdown: tokio_util::sync::CancellationToken,
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

async fn start(extra: &[(&str, &str)]) -> RunningServer {
    let mut pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_BIND", "127.0.0.1:0"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();
    for (k, v) in extra {
        pairs.insert((*k).to_string(), (*v).to_string());
    }

    let config = Config::from_map(&pairs).expect("a valid HTTP configuration");
    let state = Arc::new(ServerState::new(config).expect("server state"));
    let listener = bind("127.0.0.1:0").await.expect("a free port");
    let addr = listener.local_addr().expect("an address");

    let shutdown = tokio_util::sync::CancellationToken::new();
    let serving = shutdown.clone();
    tokio::spawn(async move {
        let _ = serve(state, listener, serving).await;
    });

    RunningServer {
        base: format!("http://{addr}"),
        shutdown,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("a client")
}

// ── discovery is reachable without the credential it helps obtain ───────────

#[tokio::test]
async fn metadata_endpoint_is_reachable_without_a_token() {
    let server = start(&[]).await;

    let response = client()
        .get(format!("{}{}", server.base, metadata::METADATA_PATH))
        .send()
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        200,
        "discovery must not require the credential it exists to help obtain"
    );

    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["resource"], "https://mcp.example.com");
    assert_eq!(body["authorization_servers"][0], "https://auth.example.com");
}

#[tokio::test]
async fn health_is_reachable_without_a_token() {
    let server = start(&[]).await;

    let response = client()
        .get(format!("{}{HEALTH_PATH}", server.base))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 200, "a load balancer holds no token");
}

// ── the MCP endpoint requires one ───────────────────────────────────────────

#[tokio::test]
async fn unauthenticated_mcp_request_returns_401_with_a_challenge() {
    let server = start(&[]).await;

    let response = client()
        .post(format!("{}{MCP_PATH}", server.base))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 401);
    let challenge = response
        .headers()
        .get("WWW-Authenticate")
        .expect("challenge header must be present or clients cannot discover auth")
        .to_str()
        .expect("utf8");

    assert!(
        challenge.contains("resource_metadata="),
        "without this a client cannot find the authorization server: {challenge}"
    );
    assert!(challenge.contains(metadata::METADATA_PATH), "{challenge}");
}

#[tokio::test]
async fn a_forged_token_is_refused_with_the_same_challenge() {
    let server = start(&[]).await;

    // Not signed by anything. It must be refused without the response saying
    // which part was wrong.
    let response = client()
        .post(format!("{}{MCP_PATH}", server.base))
        .header("Authorization", "Bearer not.a.token")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 401);
    assert!(response.headers().get("WWW-Authenticate").is_some());

    let body = response.text().await.expect("body");
    assert!(
        !body.contains("signature") && !body.contains("kid"),
        "the response must not explain the failure: {body}"
    );
}

#[tokio::test]
async fn an_empty_bearer_is_treated_as_no_bearer() {
    let server = start(&[]).await;

    let response = client()
        .post(format!("{}{MCP_PATH}", server.base))
        .header("Authorization", "Bearer    ")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn an_unknown_path_is_not_found_rather_than_unauthorized() {
    let server = start(&[]).await;

    let response = client()
        .get(format!("{}/nope", server.base))
        .send()
        .await
        .expect("response");

    // A 401 here would suggest the path exists and needs a token, which would
    // have people hunting for a credential that would not help.
    assert_eq!(response.status(), 404);
}

// ── origin and host allowlists ──────────────────────────────────────────────

#[tokio::test]
async fn the_resource_url_supplies_the_default_origin_allowlist() {
    let pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();

    let config = Config::from_map(&pairs).expect("valid");

    // rmcp disables Origin validation when the list is empty, so an unset
    // allowlist would be a hole rather than a permissive default.
    assert_eq!(config.allowed_origins, vec!["https://mcp.example.com"]);
    assert_eq!(config.allowed_hosts, vec!["mcp.example.com"]);
}

#[tokio::test]
async fn an_explicit_origin_allowlist_is_honoured() {
    let pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
        (
            "RDC_MCP_ALLOWED_ORIGINS",
            "https://app.example.com, https://other.example.com",
        ),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();

    let config = Config::from_map(&pairs).expect("valid");

    assert_eq!(
        config.allowed_origins,
        vec!["https://app.example.com", "https://other.example.com"],
        "whitespace around a comma is how everyone writes these lists"
    );
}

#[tokio::test]
async fn a_request_from_a_foreign_origin_is_rejected() {
    let server = start(&[("RDC_MCP_ALLOWED_ORIGINS", "https://app.example.com")]).await;

    // 403 specifically, and on the *metadata* endpoint — which needs no token,
    // so a 401 cannot be what produces the failure. Asserting only
    // `is_client_error` on the MCP endpoint would pass with the origin check
    // switched off entirely, since the missing token alone yields a 401.
    let response = client()
        .get(format!("{}{}", server.base, metadata::METADATA_PATH))
        .header("Origin", "https://evil.example.com")
        .send()
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        403,
        "a foreign origin must not be served, even discovery"
    );
}

#[tokio::test]
async fn a_permitted_origin_is_served() {
    let server = start(&[("RDC_MCP_ALLOWED_ORIGINS", "https://app.example.com")]).await;

    let response = client()
        .get(format!("{}{}", server.base, metadata::METADATA_PATH))
        .header("Origin", "https://app.example.com")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn an_origin_that_merely_ends_with_a_permitted_one_is_rejected() {
    // `https://evil-app.example.com` must not satisfy an allowlist naming
    // `https://app.example.com`. Suffix matching is the classic way this
    // check is got wrong.
    let server = start(&[("RDC_MCP_ALLOWED_ORIGINS", "https://app.example.com")]).await;

    for impostor in [
        "https://evil-app.example.com",
        "https://app.example.com.evil.test",
        "http://app.example.com",
    ] {
        let response = client()
            .get(format!("{}{}", server.base, metadata::METADATA_PATH))
            .header("Origin", impostor)
            .send()
            .await
            .expect("response");

        assert_eq!(response.status(), 403, "{impostor} must not be served");
    }
}

#[tokio::test]
async fn a_request_with_no_origin_is_served() {
    // Not a browser. The header exists to distinguish browsers, so its absence
    // is not something to refuse — every non-browser MCP client sends none.
    let server = start(&[("RDC_MCP_ALLOWED_ORIGINS", "https://app.example.com")]).await;

    let response = client()
        .get(format!("{}{HEALTH_PATH}", server.base))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), 200);
}
