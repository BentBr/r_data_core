#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Telling a client how to authenticate, so it does not have to be told.
//!
//! An MCP client that receives a 401 reads the `WWW-Authenticate` challenge,
//! follows the `resource_metadata` URL it names, and learns from the document
//! there which authorization server to go to. Get these two right and a
//! client configures itself from this server's URL alone; get them wrong and
//! every user configures the thing by hand, which is the whole cost of the
//! feature.
//!
//! Both are defined by RFC 9728, which the MCP authorization specification
//! adopts. Only the fields a client actually reads are emitted: an
//! unrecognised extra field is harmless, but a missing required one sends the
//! client to manual configuration.

use serde_json::{json, Value};

use crate::config::{origin_of, Config};

/// The path RFC 9728 reserves for this document.
pub const METADATA_PATH: &str = "/.well-known/oauth-protected-resource";

/// The protected-resource metadata document.
///
/// `resource` must match the audience the authorization server mints tokens
/// for, or a client will obtain a token this server then rejects — a failure
/// that looks like a bug here and is actually a configuration mismatch.
///
/// Returns an empty object when the server is not configured for OAuth, which
/// only happens in stdio mode where nothing serves this.
#[must_use]
pub fn protected_resource_document(config: &Config) -> Value {
    let (Some(resource), Some(issuer)) = (&config.resource_url, &config.oidc_issuer) else {
        return json!({});
    };

    json!({
        "resource": resource,
        "authorization_servers": [issuer],
        // Header only. Accepting a token in a query string would put it in
        // access logs and `Referer` headers.
        "bearer_methods_supported": ["header"],
        // No scopes are required beyond a valid identity: authorization is
        // decided by RDataCore from the caller's roles, not by scope. Saying
        // so explicitly stops a client inventing one and being refused.
        "scopes_supported": [],
    })
}

/// The path this server serves its metadata document on.
///
/// RFC 9728 §3.1 puts the document for a resource with a path component
/// *under* the well-known prefix: `https://host/rdc-mcp` is described at
/// `/.well-known/oauth-protected-resource/rdc-mcp`. A resource at the root
/// keeps the bare prefix.
#[must_use]
pub fn metadata_path(config: &Config) -> String {
    let suffix = config
        .resource_url
        .as_deref()
        .and_then(|raw| url::Url::parse(raw).ok())
        .map(|parsed| parsed.path().trim_end_matches('/').to_string())
        .unwrap_or_default();
    format!("{METADATA_PATH}{suffix}")
}

/// The `WWW-Authenticate` value for an unauthenticated request.
///
/// The `resource_metadata` parameter is the part that matters. Without it a
/// client knows only that it needs a bearer token, not where to get one.
#[must_use]
pub fn challenge_header(config: &Config) -> String {
    origin_of(config.resource_url.as_deref()).map_or_else(
        || "Bearer".to_string(),
        |origin| {
            format!(
                "Bearer resource_metadata=\"{origin}{}\"",
                metadata_path(config)
            )
        },
    )
}

#[cfg(test)]
mod metadata_tests;
