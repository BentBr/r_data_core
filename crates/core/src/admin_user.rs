#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod model;
#[cfg(test)]
mod model_tests;

pub use model::{AdminUser, ApiKey, UserStatus};
