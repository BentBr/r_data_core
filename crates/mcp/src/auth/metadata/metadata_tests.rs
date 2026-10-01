#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What a client must be able to read out of this server without being told
//! anything by hand.

use std::collections::HashMap;

use super::*;
use crate::config::Config;

/// A server configured for HTTP with OAuth, as production would be.
fn http_config(resource: &str, issuer: &str, audience: &str) -> Config {
    let pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_RESOURCE_URL", resource),
        ("RDC_OIDC_ISSUER", issuer),
        ("RDC_OIDC_AUDIENCE", audience),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();

    Config::from_map(&pairs).expect("a valid HTTP configuration")
}

/// A stdio server, which serves no metadata at all.
fn stdio_config() -> Config {
    let pairs: HashMap<String, String> = [
        ("RDC_BASE_URL", "https://rdc.example.com"),
        ("RDC_API_KEY", "local-development-key"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();

    Config::from_map(&pairs).expect("a valid stdio configuration")
}

#[test]
fn document_advertises_the_authorization_server_and_resource() {
    let doc = protected_resource_document(&http_config(
        "https://mcp.example.com",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    assert_eq!(doc["resource"], "https://mcp.example.com");
    assert_eq!(
        doc["authorization_servers"][0], "https://auth.example.com",
        "clients follow this to find the token endpoint"
    );
}

#[test]
fn document_accepts_bearer_tokens_only_in_the_header() {
    let doc = protected_resource_document(&http_config(
        "https://mcp.example.com",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    // A token in a query string lands in access logs and `Referer` headers.
    assert_eq!(doc["bearer_methods_supported"][0], "header");
    assert_eq!(
        doc["bearer_methods_supported"].as_array().map(Vec::len),
        Some(1),
        "no other method should be advertised"
    );
}

#[test]
fn document_asks_for_no_scopes() {
    let doc = protected_resource_document(&http_config(
        "https://mcp.example.com",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    // Authorization comes from the caller's RDataCore roles, not from scope.
    // Advertising a scope would have clients request one and be refused.
    assert_eq!(
        doc["scopes_supported"].as_array().map(Vec::len),
        Some(0),
        "expected an explicitly empty list, got {}",
        doc["scopes_supported"]
    );
}

#[test]
fn challenge_points_at_the_metadata_url() {
    let header = challenge_header(&http_config(
        "https://mcp.example.com",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    assert!(
        header.starts_with("Bearer "),
        "must be a Bearer challenge: {header}"
    );
    assert!(
        header.contains("resource_metadata="),
        "without this a client cannot discover the auth server: {header}"
    );
    assert!(
        header.contains("/.well-known/oauth-protected-resource"),
        "must name the metadata path: {header}"
    );
}

#[test]
fn the_challenge_url_has_no_doubled_slash() {
    // A resource URL with a trailing slash is the obvious thing for an
    // operator to write, and `https://mcp.example.com//.well-known/...` is not
    // the same URL — some clients fetch it verbatim and get a 404.
    let header = challenge_header(&http_config(
        "https://mcp.example.com/",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    assert!(
        header.contains("https://mcp.example.com/.well-known/oauth-protected-resource"),
        "trailing slash should not double: {header}"
    );
}

#[test]
fn the_metadata_url_is_absolute() {
    // Clients resolve it directly rather than against the request, so a
    // relative path here would send them nowhere.
    let header = challenge_header(&http_config(
        "https://mcp.example.com",
        "https://auth.example.com",
        "rdc-mcp",
    ));

    assert!(
        header.contains("\"https://"),
        "the metadata URL must be absolute: {header}"
    );
}

#[test]
fn a_stdio_server_advertises_nothing() {
    let config = stdio_config();

    // Nothing serves these in stdio mode. Emitting a half-populated document
    // would be worse than emitting none: a client would try to follow it.
    assert_eq!(protected_resource_document(&config), serde_json::json!({}));
    assert_eq!(challenge_header(&config), "Bearer");
}
