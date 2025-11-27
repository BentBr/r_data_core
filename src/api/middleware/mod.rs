mod error_handler;
mod error_handlers;

pub use error_handler::ErrorHandler;
pub use error_handlers::create_error_handlers;

// Re-export middleware from crate (for backward compatibility and tests)
#[allow(unused_imports)] // Re-exported for use in tests
pub use r_data_core_api::middleware::{
    ApiAuth, AuthMiddlewareService, CombinedAuth, ApiKeyInfo,
};
