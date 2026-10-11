#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What the authorization-code flow must not get wrong.
//!
//! The provider is a mock, so the assertions are about this side of the
//! exchange: that PKCE is present and correctly derived, that a state is
//! usable exactly once, and that `return_to` cannot carry the browser off
//! this origin.

mod exchange;
mod start;
mod state;

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
pub(super) async fn provider() -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": uri,
            "jwks_uri": format!("{uri}/jwks"),
            "authorization_endpoint": format!("{uri}/authorize"),
            "token_endpoint": format!("{uri}/token"),
        })))
        .mount(&server)
        .await;
    server
}

pub(super) fn flow(server: &MockServer, extra: &[(&str, &str)]) -> OidcFlow {
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
    OidcFlow::new(runtime, cache, None).expect("flow")
}

/// The query parameters of a redirect URL.
pub(super) fn query_of(url: &str) -> HashMap<String, String> {
    url.split_once('?')
        .expect("a redirect carries a query")
        .1
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

pub(super) fn decode(value: &str) -> String {
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

/// A provider whose discovery document omits the token endpoint.
pub(super) async fn provider_without_token_endpoint() -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": uri,
            "jwks_uri": format!("{uri}/jwks"),
            "authorization_endpoint": format!("{uri}/authorize"),
        })))
        .mount(&server)
        .await;
    server
}

/// Start a sign-in and complete it, returning whatever the flow decided.
pub(super) async fn round_trip(flow: &OidcFlow) -> Result<CompletedLogin, FlowError> {
    let redirect = flow.start(None).await.expect("start");
    flow.complete(&redirect.state, "an-authorization-code", 1800)
        .await
}

/// A flow whose interface lives at another origin.
pub(super) fn flow_with_frontend(server: &MockServer, frontend: Option<&str>) -> OidcFlow {
    let config = OidcConfig::from_map(
        &[
            ("RDC_OIDC_ISSUER".to_string(), server.uri()),
            ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
            ("RDC_OIDC_CLIENT_ID".to_string(), "rdc".to_string()),
            (
                "RDC_OIDC_REDIRECT_URI".to_string(),
                "https://rdc.example.com/cb".to_string(),
            ),
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
        Arc::new(service(
            Arc::new(FakeIdentities::default()),
            Arc::new(FakeUsers::default()),
        )),
        Arc::clone(&cache),
    ));
    OidcFlow::new(runtime, cache, frontend.map(str::to_string)).expect("flow")
}
