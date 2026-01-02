#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api;
pub mod jwt;
pub mod models;
pub mod tool_service;

pub use api::call_verification_api;
pub use jwt::{create_license_key, verify_license_key, Error};
pub use models::{LicenseClaims, LicenseType};
pub use tool_service::{
    LicenseCheckResult, LicenseCheckState, LicenseCreationResult, LicenseToolService,
    LicenseVerificationDisplayResult,
};
