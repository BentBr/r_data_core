#![allow(clippy::expect_used)]

//! Configuration tests, weighted towards the combinations that must be refused.
//!
//! A misconfigured MCP server does not fail loudly at the point of damage — it
//! serves happily while authenticating everyone as the wrong person. So the
//! refusals matter more than the successes, and get more tests.

use super::*;

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn stdio() -> Vec<(&'static str, &'static str)> {
    vec![
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_API_KEY", "secret"),
    ]
}

fn http_oidc() -> Vec<(&'static str, &'static str)> {
    vec![
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
    ]
}

// ── accepted configurations ─────────────────────────────────────────────────

#[test]
fn stdio_with_a_credential_is_valid() {
    let cfg = Config::from_map(&env(&stdio())).expect("valid stdio config");
    assert!(matches!(cfg.transport, Transport::Stdio));
    assert_eq!(cfg.base_url.as_str(), "https://rdc.example.com/");
    assert!(cfg.credential.is_some());
    assert_eq!(cfg.timeout, Duration::from_secs(30));
}

#[test]
fn http_with_an_issuer_and_audience_is_valid() {
    let cfg = Config::from_map(&env(&http_oidc())).expect("valid http config");
    assert_eq!(
        cfg.transport,
        Transport::Http {
            bind: "127.0.0.1:8931".to_string()
        },
        "an unset bind must default to loopback, not to every interface"
    );
}

// ── refusals ────────────────────────────────────────────────────────────────

#[test]
fn missing_base_url_is_rejected() {
    let err =
        Config::from_map(&env(&[("RDC_API_KEY", "secret")])).expect_err("base url is required");
    assert_eq!(err, ConfigError::MissingBaseUrl);
}

#[test]
fn a_malformed_base_url_is_rejected() {
    let err = Config::from_map(&env(&[
        ("RDC_BASE_URL", "not a url"),
        ("RDC_API_KEY", "secret"),
    ]))
    .expect_err("malformed url");
    assert!(matches!(err, ConfigError::InvalidBaseUrl(_)));
}

#[test]
fn stdio_without_a_credential_is_rejected() {
    let err = Config::from_map(&env(&[("RDC_BASE_URL", "https://rdc.example.com")]))
        .expect_err("stdio needs a credential");
    assert_eq!(err, ConfigError::MissingCredential);
}

#[test]
fn http_with_no_authentication_at_all_is_rejected() {
    // The hole an earlier draft left open: neither credential nor issuer fell
    // through a catch-all arm and started an OPEN server on a port.
    let err = Config::from_map(&env(&[
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
    ]))
    .expect_err("an unauthenticated HTTP server must never start");
    assert_eq!(err, ConfigError::HttpRequiresOidc);
}

#[test]
fn http_with_a_shared_credential_is_rejected() {
    let err = Config::from_map(&env(&[
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
        ("RDC_API_KEY", "secret"),
    ]))
    .expect_err("one shared credential collapses per-user permissions");
    assert_eq!(err, ConfigError::SharedCredentialOverHttp);
}

#[test]
fn an_issuer_without_an_audience_is_rejected() {
    let err = Config::from_map(&env(&[
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
    ]))
    .expect_err("audience is mandatory");
    assert_eq!(err, ConfigError::MissingAudience);
}

#[test]
fn http_without_a_resource_url_is_rejected() {
    let mut pairs = http_oidc();
    pairs.retain(|(k, _)| *k != "RDC_MCP_RESOURCE_URL");
    let err = Config::from_map(&env(&pairs)).expect_err("discovery needs this server's URL");
    assert_eq!(err, ConfigError::MissingResourceUrl);
}

#[test]
fn a_plaintext_resource_url_is_rejected() {
    let mut pairs = http_oidc();
    pairs.retain(|(k, _)| *k != "RDC_MCP_RESOURCE_URL");
    pairs.push(("RDC_MCP_RESOURCE_URL", "http://mcp.example.com"));
    let err = Config::from_map(&env(&pairs)).expect_err("bearer tokens over plaintext");
    assert_eq!(err, ConfigError::InsecureResourceUrl);
}

#[test]
fn a_plaintext_resource_url_on_localhost_is_allowed() {
    // Development has no certificate, and refusing here would push people
    // towards disabling the check altogether.
    for host in [
        "http://localhost:8931",
        "http://127.0.0.1:8931",
        "http://[::1]:8931",
    ] {
        let mut pairs = http_oidc();
        pairs.retain(|(k, _)| *k != "RDC_MCP_RESOURCE_URL");
        pairs.push(("RDC_MCP_RESOURCE_URL", host));
        Config::from_map(&env(&pairs))
            .unwrap_or_else(|e| panic!("{host} should be allowed for development: {e}"));
    }
}

#[test]
fn an_unknown_transport_is_rejected() {
    let mut pairs = stdio();
    pairs.push(("RDC_MCP_TRANSPORT", "carrier-pigeon"));
    let err = Config::from_map(&env(&pairs)).expect_err("unknown transport");
    assert!(matches!(err, ConfigError::InvalidTransport(t) if t == "carrier-pigeon"));
}

#[test]
fn a_non_numeric_timeout_is_rejected() {
    let mut pairs = stdio();
    pairs.push(("RDC_MCP_TIMEOUT_SECS", "soon"));
    let err = Config::from_map(&env(&pairs)).expect_err("timeout must be numeric");
    assert!(matches!(
        err,
        ConfigError::InvalidNumber("RDC_MCP_TIMEOUT_SECS", _)
    ));
}

// ── empty-string handling ───────────────────────────────────────────────────

#[test]
fn an_empty_variable_counts_as_unset() {
    // `RDC_API_KEY=` in a compose file is how people unset things far more
    // often than deleting the line. Treating it as a configured empty
    // credential would produce a baffling 401 rather than a clear refusal.
    let err = Config::from_map(&env(&[
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_API_KEY", ""),
    ]))
    .expect_err("an empty credential is not a credential");
    assert_eq!(err, ConfigError::MissingCredential);
}

#[test]
fn an_empty_transport_falls_back_to_stdio() {
    let mut pairs = stdio();
    pairs.push(("RDC_MCP_TRANSPORT", ""));
    let cfg = Config::from_map(&env(&pairs)).expect("empty transport means unset");
    assert!(matches!(cfg.transport, Transport::Stdio));
}
