#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What a discovery document must satisfy before any URL in it is followed.
//!
//! The document names the token endpoint the client secret and code go to,
//! and the key set every signature is checked against. A document from the
//! wrong issuer, or one pointing at plain-HTTP endpoints, is an attacker's
//! document until proven otherwise.

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

async fn provider_with(doc: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(doc))
        .mount(&server)
        .await;
    server
}

fn doc(issuer: &str, base: &str) -> serde_json::Value {
    json!({
        "issuer": issuer,
        "jwks_uri": format!("{base}/jwks"),
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
    })
}

#[tokio::test]
async fn a_document_from_the_configured_issuer_is_accepted() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(doc(&uri, &uri)))
        .mount(&server)
        .await;

    let found = fetch(&client().expect("client"), &uri, false)
        .await
        .expect("accepted");
    assert_eq!(found.jwks_uri, format!("{uri}/jwks"));
}

#[tokio::test]
async fn a_trailing_slash_on_either_side_does_not_matter() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(doc(&format!("{uri}/"), &uri)))
        .mount(&server)
        .await;

    fetch(&client().expect("client"), &uri, false)
        .await
        .expect("a trailing slash is not a different issuer");
}

#[tokio::test]
async fn a_document_naming_another_issuer_is_refused() {
    let server = provider_with(doc("https://somebody-else.example", "https://x.example")).await;

    let error = fetch(&client().expect("client"), &server.uri(), false)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn a_document_without_an_issuer_is_refused() {
    let server = provider_with(json!({ "jwks_uri": "https://x.example/jwks" })).await;

    let error = fetch(&client().expect("client"), &server.uri(), false)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn plain_http_endpoints_are_refused_unless_insecure_is_allowed() {
    // The mock itself is on loopback http, which is exempt; the endpoints it
    // names are on a routable host, which is not.
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(doc(&uri, "http://idp.example.internal")),
        )
        .mount(&server)
        .await;

    let http = client().expect("client");
    let error = fetch(&http, &uri, false).await.expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");

    fetch(&http, &uri, true)
        .await
        .expect("the explicit override accepts it");
}

#[tokio::test]
async fn a_non_2xx_answer_is_unreachable_not_malformed() {
    // A 500 or 404 is the provider being down or misaddressed, which is an
    // operational problem; `Malformed` is reserved for a document that
    // arrived and must not be trusted. Part B's tests assert this split.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let error = fetch(&client().expect("client"), &server.uri(), false)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Unreachable(_)), "{error}");
}

#[tokio::test]
async fn an_oversized_document_is_refused_before_being_parsed() {
    let server = MockServer::start().await;
    let huge = "x".repeat(DISCOVERY_MAX_BYTES + 1);
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_string(huge))
        .mount(&server)
        .await;

    let error = fetch(&client().expect("client"), &server.uri(), false)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn the_cache_fetches_once_within_its_ttl() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(doc(&uri, &uri)))
        .mount(&server)
        .await;

    let http = client().expect("client");
    let cache = DiscoveryCache::new();
    cache.get(&http, &uri, false).await.expect("first");
    cache.get(&http, &uri, false).await.expect("second");

    let hits = server.received_requests().await.expect("requests").len();
    assert_eq!(
        hits, 1,
        "both legs of one sign-in must see the same document"
    );
}

#[tokio::test]
async fn userinfo_cannot_disguise_a_remote_endpoint_as_loopback() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let mut document = doc(&uri, &uri);
    document["token_endpoint"] = json!("http://localhost:a@evil.example/token");
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(document))
        .mount(&server)
        .await;

    let error = fetch(&client().expect("client"), &uri, false)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn the_streaming_cap_trips_without_a_content_length() {
    // Whatever the server declares, the bytes actually read are what count.
    // A tiny cap exercises the per-chunk branch directly.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/body"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(64)))
        .mount(&server)
        .await;
    let http = client().expect("client");
    let url = format!("{}/body", server.uri());

    let error = read_capped(http.get(&url), 16, &url)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
    let ok = read_capped(http.get(&url), 64, &url).await.expect("fits");
    assert_eq!(ok.len(), 64);
}

#[tokio::test]
async fn an_oversized_key_set_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(JWKS_MAX_BYTES + 1)))
        .mount(&server)
        .await;
    let url = format!("{}/jwks", server.uri());

    let error = read_capped(client().expect("client").get(&url), JWKS_MAX_BYTES, &url)
        .await
        .expect_err("refused");
    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}
