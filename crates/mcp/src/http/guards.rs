#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The checks that run before a request reaches anything else.
//!
//! `Host` and `Origin` are whole-string comparisons. Matching a suffix would let
//! `evil-mcp.example.com` past an allowlist naming `mcp.example.com`, which is
//! the classic way this check is got wrong.

use hyper::body::Incoming;
use hyper::Request;

/// Whether a request's `Origin` is one this server serves.
///
/// An empty allowlist means the check is off, matching `rmcp`'s own
/// behaviour — a non-browser deployment has no `Origin` to check and should
/// not have to invent one. A request carrying no `Origin` passes: that is a
/// non-browser client, which the header exists to distinguish from.
pub(super) fn origin_allowed(allowed: &[String], req: &Request<Incoming>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let Some(origin) = req
        .headers()
        .get(hyper::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        return true;
    };
    allowed.iter().any(|permitted| permitted == origin)
}

/// Whether a request's `Host` is one this server answers to.
///
/// An empty allowlist means the check is off, matching `rmcp`. A request with
/// no `Host` is refused rather than waved through: HTTP/1.1 requires one, and
/// its absence on a request that reached this far is not something to
/// interpret generously.
///
/// An entry may name a bare host or a `host:port`. A bare entry matches any
/// port; an entry with a port matches only that port.
pub(super) fn host_allowed(allowed: &[String], req: &Request<Incoming>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let Some(host) = req
        .headers()
        .get(hyper::header::HOST)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    // `[::1]:8080` -> `[::1]`; `example.com:80` -> `example.com`.
    let without_port = host
        .rsplit_once(':')
        .filter(|(head, _)| !head.ends_with(':') && !head.is_empty())
        .map_or(host, |(head, _)| head);

    allowed
        .iter()
        .any(|permitted| permitted == host || permitted == without_port)
}

/// The bearer token on a request, if there is a well-formed one.
pub(super) fn bearer_of(req: &Request<Incoming>) -> Option<String> {
    req.headers()
        .get(hyper::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

/// A digest of a caller's token, for use as a cache key.
///
/// Never the token itself: a cache key ends up in logs and metrics far more
/// readily than a value does.
pub(super) fn digest(token: &str) -> String {
    use sha2::{Digest as _, Sha256};
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
