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

use crate::config::Config;

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

/// The `WWW-Authenticate` value for an unauthenticated request.
///
/// The `resource_metadata` parameter is the part that matters. Without it a
/// client knows only that it needs a bearer token, not where to get one.
#[must_use]
pub fn challenge_header(config: &Config) -> String {
    config.resource_url.as_ref().map_or_else(
        || "Bearer".to_string(),
        |resource| {
            let base = resource.trim_end_matches('/');
            format!("Bearer resource_metadata=\"{base}{METADATA_PATH}\"")
        },
    )
}

#[cfg(test)]
mod metadata_tests;
