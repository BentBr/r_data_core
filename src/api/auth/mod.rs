// Auth modules moved to r_data_core_api::auth
// Re-export for backward compatibility
#[allow(unused_imports)] // Re-exported for backward compatibility
pub use r_data_core_api::auth::{extract_and_validate_api_key, extract_and_validate_jwt};
