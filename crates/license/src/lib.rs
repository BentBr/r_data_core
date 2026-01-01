#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod jwt;
pub mod models;
pub mod tool_service;

pub use jwt::{create_license_key, verify_license_key, Error};
pub use models::{LicenseClaims, LicenseType};
pub use tool_service::LicenseToolService;
