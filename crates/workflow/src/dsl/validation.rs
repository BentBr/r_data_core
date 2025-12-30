#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use regex::Regex;
use std::collections::HashMap;

/// Validate a mapping for safe field names
///
/// # Arguments
/// * `idx` - Step index for error messages
/// * `mapping` - Mapping to validate
/// * `safe_field` - Regex pattern for safe field names
///
/// # Errors
/// Returns an error if validation fails
pub fn validate_mapping<H: std::hash::BuildHasher>(
    idx: usize,
    mapping: &HashMap<String, String, H>,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    // Allow empty mappings
    for (k, v) in mapping {
        if !safe_field.is_match(k) || !safe_field.is_match(v) {
            return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: mapping contains unsafe field names ('{k}' -> '{v}')")));
        }
    }
    Ok(())
}
