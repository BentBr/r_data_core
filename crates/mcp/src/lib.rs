#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, unsafe_code)]

//! MCP server for `RDataCore`.
//!
//! An HTTP client of the `RDataCore` admin API, exposed to AI assistants over the
//! Model Context Protocol. It holds no standing credential of its own: every
//! request carries the caller's own, so it can never exceed the permissions of
//! the human it acts for.

pub mod auth;
pub mod client;
pub mod config;
pub mod prompts;
pub mod resources;
pub mod tools;
