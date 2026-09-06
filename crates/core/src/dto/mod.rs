#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! Data-transfer objects shared between the HTTP API and API clients.
//!
//! These live in `core` rather than `api` so that client crates — notably
//! `r_data_core_mcp` — can deserialize API responses without linking
//! actix-web. `crates/api` re-exports them, so call sites there are unchanged.

pub mod dsl;
pub mod workflow;
