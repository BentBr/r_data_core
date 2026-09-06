#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What the authorization-code flow must not get wrong.
//!
//! The provider is a mock, so the assertions are about this side of the
//! exchange: that PKCE is present and correctly derived, that a state is
//! usable exactly once, and that `return_to` cannot carry the browser off
//! this origin.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::oidc::{JwkSet, OidcConfig, StaticKeySource};

use super::*;
use crate::oidc_provisioning::OidcProvisioningService;
use crate::oidc_test_fakes::{service, FakeIdentities, FakeUsers};

/// A provider that publishes endpoints but is never actually completed
/// against — these tests stop at the redirect and the state bookkeeping.
async fn provider() -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jwks_uri": format!("{uri}/jwks"),
            "authorization_endpoint": format!("{uri}/authorize"),
            "token_endpoint": format!("{uri}/token"),
        })))
        .mount(&server)
        .await;
    server
}

fn flow(server: &MockServer, extra: &[(&str, &str)]) -> OidcFlow {
    let mut pairs: Vec<(String, String)> = vec![
        ("RDC_OIDC_ISSUER".to_string(), server.uri()),
        ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
        (
            "RDC_OIDC_ROLE_MAP".to_string(),
            "rdc-ops:editor".to_string(),
        ),
        ("RDC_OIDC_CLIENT_ID".to_string(), "rdc".to_string()),
        (
            "RDC_OIDC_REDIRECT_URI".to_string(),
            "https://rdc.example.com/admin/api/v1/auth/oidc/callback".to_string(),
        ),
    ];
    pairs.extend(
        extra
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
    );

    let config = OidcConfig::from_map(&pairs.into_iter().collect())
        .expect("valid")
        .expect("enabled");
    let cache = Arc::new(CacheManager::new(CacheConfig::default()));
    let runtime = Arc::new(OidcRuntime::new(
        config,
        Arc::new(StaticKeySource::new(JwkSet::default())),
        Arc::new(service(
            Arc::new(FakeIdentities::default()),
            Arc::new(FakeUsers::default()),
        )),
        Arc::clone(&cache),
    ));
    OidcFlow::new(runtime, cache).expect("flow")
}

/// The query parameters of a redirect URL.
fn query_of(url: &str) -> HashMap<String, String> {
    url.split_once('?')
        .expect("a redirect carries a query")
        .1
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).expect("ascii");
            out.push(u8::from_str_radix(hex, 16).expect("hex"));
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("utf8")
}

// ── starting a sign-in ──────────────────────────────────────────────────────

#[tokio::test]
async fn start_sends_the_browser_to_the_provider_with_pkce_and_state() {
    let server = provider().await;
    let redirect = flow(&server, &[]).start(None).await.expect("start");
    let q = query_of(&redirect.url);

    assert!(redirect
        .url
        .starts_with(&format!("{}/authorize", server.uri())));
    assert_eq!(q.get("response_type").map(String::as_str), Some("code"));
    assert_eq!(q.get("client_id").map(String::as_str), Some("rdc"));
    assert_eq!(
        q.get("code_challenge_method").map(String::as_str),
        Some("S256"),
        "PKCE is mandatory, and plain is not PKCE"
    );
    assert!(q.contains_key("code_challenge"));
    assert_eq!(
        q.get("state").map(String::as_str),
        Some(redirect.state.as_str())
    );
}

#[tokio::test]
async fn the_challenge_is_the_hash_of_the_verifier_not_the_verifier() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow.start(None).await.expect("start");
    let q = query_of(&redirect.url);
    let challenge = q.get("code_challenge").expect("challenge");

    // Recover the verifier the flow stored and re-derive the challenge. If
    // the two ever diverge the provider rejects every exchange, and if the
    // challenge *were* the verifier, PKCE would protect nothing.
    let pending: PendingLogin = flow
        .cache
        .get(&format!("{STATE_PREFIX}{}", redirect.state))
        .await
        .expect("cache")
        .expect("stored");

    assert_eq!(
        *challenge,
        URL_SAFE_NO_PAD.encode(Sha256::digest(pending.verifier.as_bytes()))
    );
    assert_ne!(*challenge, pending.verifier);
}

#[tokio::test]
async fn every_sign_in_gets_a_fresh_state_and_verifier() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let first = flow.start(None).await.expect("first");
    let second = flow.start(None).await.expect("second");

    assert_ne!(first.state, second.state);
    assert_ne!(
        query_of(&first.url).get("code_challenge"),
        query_of(&second.url).get("code_challenge")
    );
}

#[tokio::test]
async fn the_flow_is_refused_when_no_client_is_configured() {
    let server = MockServer::start().await;
    let config = OidcConfig::from_map(
        &[
            ("RDC_OIDC_ISSUER".to_string(), server.uri()),
            ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
        ]
        .into_iter()
        .collect(),
    )
    .expect("valid")
    .expect("enabled");

    let cache = Arc::new(CacheManager::new(CacheConfig::default()));
    let runtime = Arc::new(OidcRuntime::new(
        config,
        Arc::new(StaticKeySource::new(JwkSet::default())),
        Arc::new(OidcProvisioningService::new(
            Arc::new(FakeIdentities::default()),
            Arc::new(FakeUsers::default()),
            Arc::new(crate::oidc_test_fakes::FakeRoles { roles: vec![] }),
        )),
        Arc::clone(&cache),
    ));
    let flow = OidcFlow::new(runtime, cache).expect("flow");

    assert!(!flow.is_configured());
    assert!(matches!(
        flow.start(None).await,
        Err(FlowError::NotConfigured)
    ));
}

// ── single use ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_state_we_never_issued_is_refused() {
    let server = provider().await;
    let outcome = flow(&server, &[])
        .complete("never-issued", "some-code", 1800)
        .await;

    assert!(
        matches!(outcome, Err(FlowError::UnknownState)),
        "without this check the callback is a CSRF sink"
    );
}

#[tokio::test]
async fn a_state_cannot_be_claimed_twice() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow.start(None).await.expect("start");

    // The first attempt gets past the claim and fails later, at the provider
    // — there is no real code to exchange. What matters is that it is *not*
    // refused as an unknown state.
    let first = flow.complete(&redirect.state, "code", 1800).await;
    assert!(
        !matches!(first, Err(FlowError::UnknownState)),
        "the first use of a state must be accepted"
    );

    assert!(
        matches!(
            flow.complete(&redirect.state, "code", 1800).await,
            Err(FlowError::UnknownState)
        ),
        "a state must be single-use, or an intercepted callback can be replayed"
    );
}

// ── where the browser lands ─────────────────────────────────────────────────

#[tokio::test]
async fn a_return_to_leaving_this_origin_is_discarded() {
    let server = provider().await;
    let flow = flow(&server, &[]);

    for escape in [
        "https://evil.example.com/steal",
        "//evil.example.com",
        "javascript:alert(1)",
        "not-a-path",
        // Browsers normalise a backslash to a slash, so each of these
        // resolves to another origin despite starting with a single '/'.
        // The callback appends the session tokens to whatever this returns,
        // so a miss here hands them to whoever chose the destination.
        "/\\evil.example.com",
        "/\\/evil.example.com",
        "/\\\\evil.example.com",
        // Tabs and newlines are stripped before the URL is resolved.
        "/\t/evil.example.com",
        "/\n/evil.example.com",
        "/\r\n//evil.example.com",
    ] {
        assert_eq!(
            flow.landing_path(Some(escape)),
            "/admin",
            "{escape:?} must not be somewhere this login endpoint sends a browser"
        );
    }
}

#[tokio::test]
async fn a_local_return_to_is_honoured() {
    let server = provider().await;
    assert_eq!(
        flow(&server, &[]).landing_path(Some("/admin/workflows/42")),
        "/admin/workflows/42"
    );
}

#[tokio::test]
async fn the_configured_landing_path_is_used_when_none_is_asked_for() {
    let server = provider().await;
    assert_eq!(
        flow(&server, &[("RDC_OIDC_POST_LOGIN_PATH", "/admin/dashboard")]).landing_path(None),
        "/admin/dashboard"
    );
}

#[tokio::test]
async fn a_hostile_return_to_is_not_even_stored() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow
        .start(Some("https://evil.example.com/steal"))
        .await
        .expect("start");

    let pending: PendingLogin = flow
        .cache
        .get(&format!("{STATE_PREFIX}{}", redirect.state))
        .await
        .expect("cache")
        .expect("stored");

    assert!(
        pending.return_to.is_none(),
        "validating only on the way out would leave a hostile value one bug from being used"
    );
}

// ── exchanging the code ─────────────────────────────────────────────────────

/// A provider whose discovery document omits the token endpoint.
async fn provider_without_token_endpoint() -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jwks_uri": format!("{uri}/jwks"),
            "authorization_endpoint": format!("{uri}/authorize"),
        })))
        .mount(&server)
        .await;
    server
}

/// Start a sign-in and complete it, returning whatever the flow decided.
async fn round_trip(flow: &OidcFlow) -> Result<(OidcClaims, Option<String>), FlowError> {
    let redirect = flow.start(None).await.expect("start");
    flow.complete(&redirect.state, "an-authorization-code", 1800)
        .await
}

#[tokio::test]
async fn a_provider_that_refuses_the_exchange_is_reported_without_the_code() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        // Providers routinely echo the submitted parameters into an error
        // body. None of that belongs in this server's logs.
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string("invalid_grant: an-authorization-code was already used"),
        )
        .mount(&server)
        .await;

    let error = round_trip(&flow(&server, &[]))
        .await
        .expect_err("the exchange should fail");
    assert!(
        matches!(error, FlowError::Provider(_)),
        "expected a provider error, got {error:?}"
    );
    assert!(
        !error.to_string().contains("an-authorization-code"),
        "the authorization code must not travel into the error: {error}"
    );
}

#[tokio::test]
async fn a_token_response_without_an_id_token_is_refused() {
    // Some providers answer with an opaque access token and nothing else.
    // There is no identity in that, and guessing is not an option.
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "opaque-and-useless-here",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let outcome = round_trip(&flow(&server, &[])).await;

    assert!(
        matches!(outcome, Err(FlowError::NoIdToken)),
        "expected NoIdToken, got {outcome:?}"
    );
}

#[tokio::test]
async fn an_unreadable_token_response_is_a_provider_error() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>not json</html>"))
        .mount(&server)
        .await;

    let outcome = round_trip(&flow(&server, &[])).await;

    assert!(
        matches!(outcome, Err(FlowError::Provider(_))),
        "expected a provider error, got {outcome:?}"
    );
}

#[tokio::test]
async fn a_discovery_document_without_a_token_endpoint_is_refused() {
    let server = provider_without_token_endpoint().await;
    let flow = flow(&server, &[]);

    // `start` works — the authorization endpoint is there — and the failure
    // only appears at the exchange, which is exactly the confusing case worth
    // naming precisely.
    let outcome = round_trip(&flow).await;

    assert!(
        matches!(outcome, Err(FlowError::MissingEndpoint("token_endpoint"))),
        "expected a named missing endpoint, got {outcome:?}"
    );
}

#[tokio::test]
async fn the_client_secret_is_sent_when_one_is_configured() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": "x.y.z" })))
        .mount(&server)
        .await;

    let flow = flow(&server, &[("RDC_OIDC_CLIENT_SECRET", "s3cret")]);
    // Fails later at validation — the point is what reached the provider.
    let _ = round_trip(&flow).await;

    let requests = server.received_requests().await.expect("requests");
    let body = requests
        .iter()
        .find(|r| r.url.path() == "/token")
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .expect("a token request");

    assert!(body.contains("code_verifier="), "PKCE must be sent: {body}");
    assert!(body.contains("client_secret=s3cret"), "{body}");
    assert!(body.contains("grant_type=authorization_code"), "{body}");
}

#[tokio::test]
async fn no_client_secret_is_sent_for_a_public_client() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": "x.y.z" })))
        .mount(&server)
        .await;

    let _ = round_trip(&flow(&server, &[])).await;

    let requests = server.received_requests().await.expect("requests");
    let body = requests
        .iter()
        .find(|r| r.url.path() == "/token")
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .expect("a token request");

    assert!(
        !body.contains("client_secret"),
        "a public client sends none; PKCE is what protects the exchange: {body}"
    );
}
