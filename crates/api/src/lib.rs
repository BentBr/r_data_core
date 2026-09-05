#![deny(unsafe_code)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

pub mod admin;
pub mod api_state;
pub mod api_state_impl;
pub mod auth;
pub mod client_ip;
pub mod docs;
pub mod health;
pub mod middleware;
pub mod models;
pub mod public;
pub mod query;
pub mod response;
pub mod token_service;

// Re-export commonly used types
pub use api_state::{
    configure_app, configure_app_with_options, ApiConfiguration, ApiStateTrait, ApiStateWrapper,
};
pub use api_state_impl::ApiState;
pub use response::ApiResponse;
