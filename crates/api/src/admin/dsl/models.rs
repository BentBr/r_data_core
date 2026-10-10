#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! DSL DTOs.
//!
//! The definitions live in `r_data_core_core::dto::dsl` so that API clients can
//! share them without linking actix-web. They are re-exported here to keep call
//! sites and the `OpenAPI` registration unchanged.

pub use r_data_core_core::dto::dsl::{
    DslFieldSpec, DslOptionsAndExamplesResponse, DslOptionsResponse, DslTypeSpec,
    DslValidateRequest, DslValidateResponse,
};
