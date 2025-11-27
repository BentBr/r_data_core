#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api;
pub mod entity;
pub mod error;
pub mod notification;
pub mod services;
pub use r_data_core_core::utils;
pub use r_data_core_core::versioning;

/// The version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the library
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The description of the library
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Re-export common types
pub use error::Error;
pub use r_data_core_core::error::Result;

/// API state that can be shared across handlers
pub use api::ApiState;

/// Re-export services
pub use r_data_core_services::{AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService};

use log::info;

/// Initialize the r_data_core library
pub fn init() -> r_data_core_core::error::Result<()> {
    info!("Initializing r_data_core...");
    Ok(())
}
