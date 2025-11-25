#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod entities;
pub mod queries;
pub mod dynamic_entities;

// Routes remain in src/api/public/*/routes.rs in the main crate
// as they need access to concrete ApiState and service types
// This module provides the structure and models for public routes

