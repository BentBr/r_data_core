// Middleware modules moved to r_data_core_api::middleware
// Re-export for backward compatibility
#[allow(unused_imports)] // Re-exported for backward compatibility
pub use r_data_core_api::middleware::{
    ApiAuth, AuthMiddlewareService, CombinedAuth, ApiKeyInfo, ErrorHandler, create_error_handlers,
};
