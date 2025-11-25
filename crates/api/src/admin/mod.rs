#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api_keys;
pub mod auth;
pub mod dsl;
pub mod entity_definitions;
pub mod permissions;
pub mod system;
pub mod workflows;

// Routes remain in src/api/admin/*/routes.rs in the main crate
// as they need access to concrete ApiState and service types
// This module provides the structure and models for admin routes

