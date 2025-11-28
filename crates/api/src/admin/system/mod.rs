#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod models;
pub mod routes;

pub use routes::register_routes;

// Routes remain in src/api/admin/system/routes.rs in the main crate
