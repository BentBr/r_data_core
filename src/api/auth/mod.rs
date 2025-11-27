pub mod permission_check;
pub mod utils;

// Re-export common types and functions (for backward compatibility)
// These are now available from r_data_core_api::auth
#[allow(unused_imports)] // Re-exported for backward compatibility
pub use utils::{extract_and_validate_api_key, extract_and_validate_jwt};
