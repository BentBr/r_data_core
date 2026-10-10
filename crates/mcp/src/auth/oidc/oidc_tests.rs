#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The guarantees this backend exists to hold.
//!
//! Three of these are worth more than the rest. `never_forwards_the_inbound_token`
//! guards a specification requirement. `never_falls_back_to_a_static_credential`
//! guards the claim that the server cannot act as anyone but its caller. And
//! `two_callers_never_share_a_local_token` guards the token cache, which is the
//! natural place for an impersonation bug to appear.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

/// A clock the test moves by hand, so expiry is deterministic.
struct TestClock(AtomicU64);

impl TestClock {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicU64::new(1_000)))
    }
}

impl Clock for TestClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn config_for(base_url: &str) -> Config {
    let pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", base_url),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();

    Config::from_map(&pairs).expect("a valid HTTP configuration")
}

fn config() -> Config {
    config_for("https://rdc.example.com")
}

fn validated_ctx(bearer: &str) -> CallerContext {
    CallerContext {
        bearer: Some(bearer.to_string()),
        validated: true,
    }
}

/// The exchange endpoint, answering with a fixed local token.
async fn mock_exchange(server: &MockServer, local_token: &str, expires_in: u64) {
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/oidc/exchange"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "Success",
            "message": "ok",
            "meta": null,
            "data": { "access_token": local_token, "token_type": "Bearer", "expires_in": expires_in }
        })))
        .mount(server)
        .await;
}

fn authorization(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .find(|(k, _)| k == "Authorization")
        .map(|(_, v)| v.clone())
        .expect("an authorization header")
}

// ── the exchange ────────────────────────────────────────────────────────────

#[tokio::test]
async fn exchanges_the_identity_for_a_local_token() {
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token-for-alice", 1800).await;

    let backend = OidcBackend::new(&config_for(&rdc.uri())).expect("backend");
    let headers = backend
        .outbound_headers(&validated_ctx("alice-idp-token"))
        .await
        .expect("headers");

    assert_eq!(
        authorization(&headers),
        "Bearer local-token-for-alice",
        "the LOCAL token must be sent"
    );
}

#[tokio::test]
async fn never_forwards_the_inbound_token() {
    // The MCP authorization specification names token passthrough as an
    // anti-pattern and disallows it. This test is the guard; do not delete it.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token", 1800).await;

    let backend = OidcBackend::new(&config_for(&rdc.uri())).expect("backend");
    let headers = backend
        .outbound_headers(&validated_ctx("alice-idp-token"))
        .await
        .expect("headers");

    assert!(
        !format!("{headers:?}").contains("alice-idp-token"),
        "the caller's IdP token must never leave this process: {headers:?}"
    );
}

#[tokio::test]
async fn a_refused_exchange_is_reported_without_echoing_the_token() {
    let rdc = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/oidc/exchange"))
        // A provider or gateway that echoes the presented token into an error
        // body must not get it written into this server's logs.
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice-idp-token"))
        .mount(&rdc)
        .await;

    let backend = OidcBackend::new(&config_for(&rdc.uri())).expect("backend");
    let error = backend
        .outbound_headers(&validated_ctx("alice-idp-token"))
        .await
        .expect_err("a refusal");

    assert!(
        !error.to_string().contains("alice-idp-token"),
        "the error must not carry the token: {error}"
    );
}

// ── the refusals ────────────────────────────────────────────────────────────

#[tokio::test]
async fn rejects_an_unvalidated_context() {
    // Defence in depth: if the HTTP layer ever fails to validate, the backend
    // must refuse rather than exchange an unchecked token.
    let backend = OidcBackend::new(&config()).expect("backend");
    let error = backend
        .outbound_headers(&CallerContext {
            bearer: Some("unchecked".to_string()),
            validated: false,
        })
        .await
        .expect_err("must refuse an unvalidated context");

    assert!(matches!(error, AuthError::InvalidToken(_)), "got {error:?}");
}

#[tokio::test]
async fn rejects_a_request_with_no_bearer() {
    let backend = OidcBackend::new(&config()).expect("backend");
    let error = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect_err("no anonymous access");

    assert!(
        matches!(error, AuthError::MissingCredential),
        "got {error:?}"
    );
}

#[tokio::test]
async fn never_falls_back_to_a_static_credential() {
    // The guarantee the whole permission model rests on: with no valid caller
    // the backend produces nothing — it does not reach for a configured key.
    let mut cfg = config();
    cfg.credential = Some("a-service-key".to_string());
    let backend = OidcBackend::new(&cfg).expect("backend");

    let result = backend.outbound_headers(&CallerContext::default()).await;

    assert!(result.is_err(), "must not authenticate as anyone else");
    if let Ok(headers) = result {
        assert!(
            !format!("{headers:?}").contains("a-service-key"),
            "a static credential must never appear in an OIDC-mode request"
        );
    }
}

// ── the cache ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn caches_the_local_token_and_does_not_exchange_on_every_call() {
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token", 1800).await;
    let backend = OidcBackend::new(&config_for(&rdc.uri())).expect("backend");

    for _ in 0..5 {
        backend
            .outbound_headers(&validated_ctx("alice"))
            .await
            .expect("headers");
    }

    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        1,
        "an exchange per outbound call would triple every request's latency"
    );
}

#[tokio::test]
async fn two_callers_never_share_a_local_token() {
    // A cache keyed on anything but the caller is an impersonation bug.
    let rdc = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/oidc/exchange"))
        .respond_with(|request: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).expect("json");
            let caller = body["token"].as_str().unwrap_or("unknown");
            ResponseTemplate::new(200).set_body_json(json!({
                "status": "Success",
                "message": "ok",
                "meta": null,
                "data": { "access_token": format!("local-for-{caller}"), "expires_in": 1800 }
            }))
        })
        .mount(&rdc)
        .await;

    let backend = OidcBackend::new(&config_for(&rdc.uri())).expect("backend");

    let alice = backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("alice");
    let bob = backend
        .outbound_headers(&validated_ctx("bob"))
        .await
        .expect("bob");

    assert_eq!(authorization(&alice), "Bearer local-for-alice");
    assert_eq!(authorization(&bob), "Bearer local-for-bob");
    assert_ne!(alice, bob, "each caller must get their own local token");
}

#[tokio::test]
async fn a_stale_cached_token_is_refreshed() {
    // Exchanged tokens are short-lived; serving an expired one produces a
    // confusing 401 from RDataCore rather than a clean re-exchange.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token", 100).await;

    let clock = TestClock::new();
    let backend = OidcBackend::with_clock(&config_for(&rdc.uri()), clock.clone()).expect("backend");

    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("first");

    // Still inside the usable window (80 of 100 seconds).
    clock.0.store(1_070, Ordering::SeqCst);
    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("second");
    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        1,
        "a token well inside its life must not be re-exchanged"
    );

    // Past the refresh point, but before the token itself would expire — the
    // whole purpose of refreshing early.
    clock.0.store(1_090, Ordering::SeqCst);
    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("third");
    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        2,
        "a token past its refresh point must be exchanged again"
    );
}

#[tokio::test]
async fn an_exchange_without_a_stated_lifetime_still_expires() {
    // A response that omits `expires_in` must not produce a token cached for
    // ever; the fallback is deliberately short.
    let rdc = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/oidc/exchange"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "Success",
            "message": "ok",
            "meta": null,
            "data": { "access_token": "local-token" }
        })))
        .mount(&rdc)
        .await;

    let clock = TestClock::new();
    let backend = OidcBackend::with_clock(&config_for(&rdc.uri()), clock.clone()).expect("backend");

    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("first");
    clock
        .0
        .store(1_000 + FALLBACK_LIFETIME_SECS, Ordering::SeqCst);
    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("second");

    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        2,
        "an unstated lifetime must not mean an unlimited one"
    );
}

#[tokio::test]
async fn aged_out_entries_are_dropped_rather_than_accumulating() {
    // Nothing else removes an entry, so without pruning a long-running server
    // keeps one per distinct token it has ever seen — and tokens rotate, so
    // that set only ever grows.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token", 100).await;

    let clock = TestClock::new();
    let backend = OidcBackend::with_clock(&config_for(&rdc.uri()), clock.clone()).expect("backend");

    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("alice");
    assert_eq!(backend.cache.read().await.len(), 1);

    // Well past alice's refresh point, so her entry is dead weight.
    clock.0.store(1_200, Ordering::SeqCst);
    backend
        .outbound_headers(&validated_ctx("bob"))
        .await
        .expect("bob");

    assert_eq!(
        backend.cache.read().await.len(),
        1,
        "alice's stale entry should have been evicted when bob's was written"
    );
}

#[tokio::test]
async fn a_live_entry_is_not_evicted_by_another_callers_arrival() {
    // The pruning must be narrow: evicting a live entry would mean an extra
    // exchange per caller whenever anyone else connects.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "local-token", 1800).await;

    let clock = TestClock::new();
    let backend = OidcBackend::with_clock(&config_for(&rdc.uri()), clock.clone()).expect("backend");

    backend
        .outbound_headers(&validated_ctx("alice"))
        .await
        .expect("alice");
    backend
        .outbound_headers(&validated_ctx("bob"))
        .await
        .expect("bob");

    assert_eq!(
        backend.cache.read().await.len(),
        2,
        "both callers are within their token's life and should both be held"
    );
}
