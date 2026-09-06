#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Trusting identities from an external OIDC provider.
//!
//! This module is **pure**: configuration, token validation and claim-to-role
//! mapping, with no HTTP and no database. `core` is the bottom of the
//! dependency graph and has no `reqwest`; fetching key sets belongs in
//! `services`, behind the [`keys::KeySource`] trait declared here.
//!
//! OIDC is not a parallel authorization path. It is a third way to populate
//! `AuthUserClaims` — validate, resolve the user, map roles, mint the same
//! claims every existing route already reads. That is what keeps the change
//! contained.

pub mod config;
pub mod keys;
pub mod role_mapping;
pub mod validation;

pub use config::{OidcConfig, OidcConfigError};
pub use keys::{Jwk, JwkSet, KeySource, KeySourceError, OidcClaims, StaticKeySource};
pub use role_mapping::{map_roles, MappingError};
pub use validation::{validate_token, ValidationError};
