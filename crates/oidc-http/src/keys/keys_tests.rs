#![allow(clippy::expect_used)]

//! The tests that matter here are the caching and the rate limit: one is a
//! performance property, the other stops this server being turned into a
//! weapon aimed at the identity provider.

use std::collections::HashMap;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn config_for(server: &MockServer, ttl_secs: u64) -> OidcConfig {
    let pairs: HashMap<String, String> = [
        ("RDC_OIDC_ISSUER".to_string(), server.uri()),
        ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
        ("RDC_OIDC_JWKS_TTL_SECS".to_string(), ttl_secs.to_string()),
    ]
    .into_iter()
    .collect();
    OidcConfig::from_map(&pairs)
        .expect("valid")
        .expect("enabled")
}

fn jwk(kid: &str) -> serde_json::Value {
    json!({ "kid": kid, "kty": "RSA", "alg": "RS256", "n": "abc", "e": "AQAB" })
}

async fn mock_provider(server: &MockServer, kids: &[&str]) {
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jwks_uri": format!("{}/jwks", server.uri())
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "keys": kids.iter().map(|k| jwk(k)).collect::<Vec<_>>()
        })))
        .mount(server)
        .await;
}

async fn jwks_requests(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .expect("requests")
        .iter()
        .filter(|r| r.url.path() == "/jwks")
        .count()
}

#[tokio::test]
async fn keys_are_fetched_and_then_cached() {
    let server = MockServer::start().await;
    mock_provider(&server, &["key-1"]).await;
    let source = HttpKeySource::new(&config_for(&server, 3600)).expect("source");

    let first = source.keys().await.expect("first fetch");
    let second = source.keys().await.expect("cached");

    assert_eq!(first, second);
    assert_eq!(
        jwks_requests(&server).await,
        1,
        "the second call must be served from cache"
    );
}

#[tokio::test]
async fn an_expired_cache_is_refetched() {
    let server = MockServer::start().await;
    mock_provider(&server, &["key-1"]).await;
    // A zero TTL makes every read a miss.
    let source = HttpKeySource::new(&config_for(&server, 0)).expect("source");

    let _ = source.keys().await.expect("first");
    let _ = source.keys().await.expect("second");

    assert_eq!(jwks_requests(&server).await, 2);
}

#[tokio::test]
async fn repeated_unknown_kids_trigger_exactly_one_refetch() {
    // Without this limit an attacker sending tokens with random key ids turns
    // this server into a load generator aimed at the identity provider.
    let server = MockServer::start().await;
    mock_provider(&server, &["key-1"]).await;
    let source = HttpKeySource::new(&config_for(&server, 3600)).expect("source");

    let _ = source.keys().await.expect("prime the cache");
    let before = jwks_requests(&server).await;

    for _ in 0..10 {
        let _ = source.refresh_for_unknown_kid("forged-kid").await;
    }

    assert_eq!(
        jwks_requests(&server).await - before,
        1,
        "ten unknown key ids must cost the provider one request, not ten"
    );
}

#[tokio::test]
async fn a_refresh_is_skipped_when_the_key_is_already_present() {
    // A concurrent refresh may have already fetched it; re-fetching would be
    // pure waste.
    let server = MockServer::start().await;
    mock_provider(&server, &["key-1"]).await;
    let source = HttpKeySource::new(&config_for(&server, 3600)).expect("source");

    let _ = source.keys().await.expect("prime");
    let before = jwks_requests(&server).await;

    let keys = source
        .refresh_for_unknown_kid("key-1")
        .await
        .expect("already present");

    assert!(keys.find("key-1").is_some());
    assert_eq!(jwks_requests(&server).await, before, "no refetch needed");
}

#[tokio::test]
async fn an_unreachable_provider_is_reported_not_silently_empty() {
    // Returning an empty key set would make every token fail with "unknown
    // key", which points an operator at the wrong problem entirely.
    let server = MockServer::start().await;
    // Discovery is not mocked, so it 404s.
    let source = HttpKeySource::new(&config_for(&server, 3600)).expect("source");

    let err = source.keys().await.expect_err("discovery should fail");
    assert!(
        matches!(
            err,
            KeySourceError::Malformed(_) | KeySourceError::Unreachable(_)
        ),
        "got {err:?}"
    );
}

#[tokio::test]
async fn a_trailing_slash_on_the_issuer_does_not_break_discovery() {
    // Operators copy issuer URLs with and without one, and a double slash
    // produces a 404 that looks like a provider outage.
    let server = MockServer::start().await;
    mock_provider(&server, &["key-1"]).await;

    let pairs: HashMap<String, String> = [
        ("RDC_OIDC_ISSUER".to_string(), format!("{}/", server.uri())),
        ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
    ]
    .into_iter()
    .collect();
    let config = OidcConfig::from_map(&pairs)
        .expect("valid")
        .expect("enabled");

    let source = HttpKeySource::new(&config).expect("source");
    source.keys().await.expect("discovery should still work");
}
