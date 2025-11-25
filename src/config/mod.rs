#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Re-export config loader from the core (used in library code)
pub use r_data_core_core::config::load_app_config;
