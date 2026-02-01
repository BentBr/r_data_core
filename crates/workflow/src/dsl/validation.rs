#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use regex::Regex;
use std::collections::HashMap;

use super::execution::LITERAL_PREFIX;

/// Check if a mapping value is a valid literal value
///
/// Literal values start with `@literal:` followed by valid JSON.
fn is_valid_literal_value(value: &str) -> bool {
    if !value.starts_with(LITERAL_PREFIX) {
        return false;
    }

    let json_str = &value[LITERAL_PREFIX.len()..];
    serde_json::from_str::<serde_json::Value>(json_str).is_ok()
}

/// Validate a mapping for safe field names
///
/// # Arguments
/// * `idx` - Step index for error messages
/// * `mapping` - Mapping to validate
/// * `safe_field` - Regex pattern for safe field names
///
/// # Errors
/// Returns an error if validation fails
///
/// # Mapping Value Types
/// - Field reference: Must match `safe_field` regex (e.g., `field_name`, `nested.field`)
/// - Literal value: Must start with `@literal:` followed by valid JSON (e.g., `@literal:true`)
pub fn validate_mapping<H: std::hash::BuildHasher>(
    idx: usize,
    mapping: &HashMap<String, String, H>,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    // Allow empty mappings
    for (k, v) in mapping {
        // Destination (key) must always be a safe field name
        if !safe_field.is_match(k) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: mapping destination contains unsafe field name ('{k}')"
            )));
        }

        // Source (value) can be either:
        // 1. A safe field name (e.g., "field_name")
        // 2. A literal value (e.g., "@literal:true")
        if !safe_field.is_match(v) && !is_valid_literal_value(v) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: mapping source '{v}' is neither a valid field name nor a literal value"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_field_regex() -> Regex {
        Regex::new(r"^[A-Za-z_][A-Za-z0-9_\.]*$").unwrap()
    }

    #[test]
    fn test_validate_mapping_field_reference() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("target".to_string(), "source_field".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_nested_field() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("target".to_string(), "nested.field.path".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_literal_true() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("published".to_string(), "@literal:true".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_literal_false() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("active".to_string(), "@literal:false".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_literal_string() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("status".to_string(), "@literal:\"active\"".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_literal_number() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("count".to_string(), "@literal:42".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_literal_null() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("data".to_string(), "@literal:null".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }

    #[test]
    fn test_validate_mapping_invalid_literal() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("field".to_string(), "@literal:invalid".to_string());

        let result = validate_mapping(0, &mapping, &safe_field);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mapping_unsafe_destination() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("@unsafe".to_string(), "source".to_string());

        let result = validate_mapping(0, &mapping, &safe_field);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mapping_mixed() {
        let safe_field = safe_field_regex();
        let mut mapping = HashMap::new();
        mapping.insert("field1".to_string(), "source_field".to_string());
        mapping.insert("published".to_string(), "@literal:true".to_string());
        mapping.insert("count".to_string(), "@literal:0".to_string());

        assert!(validate_mapping(0, &mapping, &safe_field).is_ok());
    }
}
