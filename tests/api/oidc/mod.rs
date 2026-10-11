#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for the OIDC authentication arm.

pub mod callback_e2e_tests;
pub mod harness;
pub mod login_flow_tests;
pub mod refresh_tests;
pub mod security_tests;
pub mod sso_user_edit_tests;
pub mod validation_tests;
