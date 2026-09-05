#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod handlers;
pub mod helpers;

// Re-export everything including utoipa __path_* types needed by docs/mod.rs
pub use handlers::*;
