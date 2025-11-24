#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api_state;

pub mod auth;
pub mod health;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod query;
pub mod response;

// Re-export commonly used types
pub use api_state::{ApiConfiguration, ApiStateTrait, configure_app, configure_app_with_options};
pub use response::ApiResponse;
