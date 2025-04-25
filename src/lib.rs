pub mod api;
pub mod cache;
pub mod config;
pub mod db;
pub mod entity;
pub mod error;
pub mod notification;
pub mod services;
pub mod versioning;
pub mod workflow;

/// The version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the library
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The description of the library
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Re-export common types
pub use error::{Error, Result};

/// API state that can be shared across handlers
pub use api::ApiState;

/// Re-export services
pub use crate::services::{AdminUserService, ApiKeyService, ClassDefinitionService};

use log::info;

/// Initialize the r_data_core library
pub fn init() -> Result<()> {
    info!("Initializing r_data_core...");
    Ok(())
}
