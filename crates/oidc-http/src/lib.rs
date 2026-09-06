#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Talking to an OIDC provider over HTTP.
//!
//! `core` declares the `KeySource` trait and validates tokens, but is a domain
//! leaf with no HTTP dependency and must stay that way. The implementation
//! that actually fetches a provider's discovery document and signing keys has
//! to live somewhere above it.
//!
//! It lives in its own crate rather than in `services` because two very
//! different things need it: `RDataCore` itself, and the MCP server. The MCP
//! server is an HTTP client of `RDataCore` and has no business linking the
//! service or persistence layers to reach one cache — the workspace's layering
//! test enforces exactly that. A second copy of this code would be worse
//! still: the rate limit in `keys` is what stops a stream of unknown key ids
//! turning either server into a load generator aimed at the identity provider,
//! and a duplicate is one refactor away from losing it.

pub mod discovery;
pub mod keys;

pub use discovery::{client, fetch, Discovery, FETCH_TIMEOUT};
pub use keys::HttpKeySource;
