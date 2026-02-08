#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests for entity definition field constraints via the HTTP API.
//!
//! Tests cover:
//! - Creating entity definitions with nested constraints structure
//! - Retrieving entity definitions with constraints preserved
//! - Unique field constraints and index generation
//! - Pattern/regex constraints
//! - Min/max length constraints for strings
//! - Min/max value constraints for numeric fields

pub mod common;
pub mod edge_cases_tests;
pub mod enum_constraints_tests;
pub mod numeric_constraints_tests;
pub mod string_constraints_tests;
pub mod unique_field_tests;
pub mod update_constraints_tests;
