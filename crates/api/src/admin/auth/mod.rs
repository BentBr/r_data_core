#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod models;

// Routes remain in src/api/admin/auth/routes.rs in the main crate
// as they need access to concrete ApiState and service types

