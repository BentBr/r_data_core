#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod models;
pub mod repository;
pub mod routes;

pub use routes::register_routes;
