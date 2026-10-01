#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod helpers;
mod orchestration;
mod rate_limit;
pub mod routes;

pub use routes::register_routes;
