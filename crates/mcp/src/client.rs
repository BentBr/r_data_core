#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Typed access to the `RDataCore` admin API.

pub mod error;

pub use error::ClientError;
