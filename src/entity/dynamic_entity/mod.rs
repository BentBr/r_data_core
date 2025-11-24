//! Dynamic entity module provides interfaces for working with dynamic data

pub mod validator;

// Re-export commonly used types
pub use validator::validate_entity_with_violations;
