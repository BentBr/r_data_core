#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod entities;
pub mod queries;
pub mod workflows;
pub mod dynamic_entities;

// Dynamic entities routes remain in src/api/public/dynamic_entities/routes.rs
// as they need access to validator functions from the main crate

