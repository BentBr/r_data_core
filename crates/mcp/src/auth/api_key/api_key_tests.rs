#![allow(clippy::expect_used, clippy::unwrap_used)]

//! That the key is exchanged rather than sent.
//!
//! The previous version of this backend attached `X-API-Key` to every
//! outbound request. The admin API requires a JWT, so every one of those
//! requests was rejected — and the tests that would have shown it existed but
//! had never been run. `the_key_never_goes_to_an_admin_endpoint` is the guard
//! against that returning.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

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
    let pairs: HashMap<String, String> = [("RDC_BASE_URL", base_url), ("RDC_API_KEY", "secret")]
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    Config::from_map(&pairs).expect("a valid stdio configuration")
}

/// The exchange endpoint, answering with an admin token.
async fn mock_exchange(server: &MockServer, token: &str, expires_in: u64) {
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/api-key/token"))
        .and(header("X-API-Key", "secret-value"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "Success",
            "message": "ok",
            "meta": null,
            "data": { "access_token": token, "token_type": "Bearer", "expires_in": expires_in }
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

#[tokio::test]
async fn exchanges_the_key_for_an_admin_token() {
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "an-admin-token", 1800).await;

    let backend =
        ApiKeyBackend::new(&config_for(&rdc.uri()), "secret-value".to_string()).expect("backend");
    let headers = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("headers");

    assert_eq!(authorization(&headers), "Bearer an-admin-token");
}

#[tokio::test]
async fn the_key_never_goes_to_an_admin_endpoint() {
    // The admin API requires a JWT. Sending the key itself is what the
    // previous version did, and every admin route rejected it.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "an-admin-token", 1800).await;

    let backend =
        ApiKeyBackend::new(&config_for(&rdc.uri()), "secret-value".to_string()).expect("backend");
    let headers = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("headers");

    assert!(
        !format!("{headers:?}").contains("secret-value"),
        "the key must stay at the exchange: {headers:?}"
    );
    assert!(
        !headers.iter().any(|(k, _)| k == "X-API-Key"),
        "got {headers:?}"
    );
}

#[tokio::test]
async fn attaches_nothing_else() {
    // An extra header here would be sent on every request; keep the surface
    // exactly one header wide.
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "an-admin-token", 1800).await;

    let backend =
        ApiKeyBackend::new(&config_for(&rdc.uri()), "secret-value".to_string()).expect("backend");
    let headers = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("headers");

    assert_eq!(headers.len(), 1, "got {headers:?}");
}

#[tokio::test]
async fn the_token_is_reused_rather_than_re_exchanged() {
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "an-admin-token", 1800).await;

    let backend =
        ApiKeyBackend::new(&config_for(&rdc.uri()), "secret-value".to_string()).expect("backend");
    for _ in 0..5 {
        backend
            .outbound_headers(&CallerContext::default())
            .await
            .expect("headers");
    }

    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        1,
        "an exchange per call would double every request"
    );
}

#[tokio::test]
async fn a_stale_token_is_exchanged_again() {
    let rdc = MockServer::start().await;
    mock_exchange(&rdc, "an-admin-token", 100).await;

    let clock = TestClock::new();
    let backend = ApiKeyBackend::with_clock(
        &config_for(&rdc.uri()),
        "secret-value".to_string(),
        clock.clone(),
    )
    .expect("backend");

    backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("first");
    clock.0.store(1_090, Ordering::SeqCst);
    backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("second");

    assert_eq!(
        rdc.received_requests().await.expect("requests").len(),
        2,
        "a token past its refresh point must be exchanged again"
    );
}

#[tokio::test]
async fn a_revoked_key_is_reported_as_such() {
    let rdc = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/api/v1/auth/api-key/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("nope: secret-value"))
        .mount(&rdc)
        .await;

    let backend =
        ApiKeyBackend::new(&config_for(&rdc.uri()), "secret-value".to_string()).expect("backend");
    let error = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect_err("a refusal");

    assert!(
        error.to_string().contains("RDC_API_KEY"),
        "the message should name what to check: {error}"
    );
    assert!(
        !error.to_string().contains("secret-value"),
        "and must not echo the key: {error}"
    );
}
