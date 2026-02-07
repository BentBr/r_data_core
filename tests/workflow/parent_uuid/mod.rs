#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `parent_uuid` resolution and path derivation in workflows.
//!
//! This module contains tests organized by scope:
//! - `resolve_entity_path_tests` - Tests for the `resolve_entity_path` function
//! - `path_derivation_tests` - Tests for path auto-generation from `parent_uuid`
//! - `data_integrity_tests` - Tests ensuring path-parent relationship integrity

pub mod data_integrity_tests;
pub mod helpers;
pub mod path_derivation_tests;
pub mod resolve_entity_path_tests;
