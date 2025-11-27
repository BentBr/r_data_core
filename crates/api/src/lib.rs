#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod admin;
pub mod api_state;
pub mod api_state_impl;
pub mod auth;
pub mod docs;
pub mod health;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod public;
pub mod query;
pub mod response;

// Re-export commonly used types
pub use api_state::{ApiConfiguration, ApiStateTrait, ApiStateWrapper, configure_app, configure_app_with_options};
pub use api_state_impl::ApiState;
pub use response::ApiResponse;
