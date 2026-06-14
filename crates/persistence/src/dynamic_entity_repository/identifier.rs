use crate::dynamic_entity_utils::SYSTEM_FIELDS;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::{Error, Result};

/// Synthetic filter keys handled specially by the query builder (not real columns).
const SYNTHETIC_FILTER_KEYS: &[&str] = &["path_prefix", "path_equals"];

/// Validate that `field` is a known identifier for `entity_def`, returning a
/// safely double-quoted SQL identifier. Rejects anything not in the system-field
/// allowlist or the entity definition.
///
/// # Errors
/// Returns [`Error::Validation`] if the identifier is unknown or malformed.
pub fn validate_and_quote(field: &str, entity_def: &EntityDefinition) -> Result<String> {
    if is_known_field(field, entity_def) {
        Ok(quote_ident(field))
    } else {
        Err(Error::Validation(format!(
            "Unknown field identifier: {field}"
        )))
    }
}

/// Like [`validate_and_quote`] but for filter keys, which may be synthetic.
///
/// # Errors
/// Returns [`Error::Validation`] if the identifier is unknown.
pub fn validate_filter_key(field: &str, entity_def: &EntityDefinition) -> Result<()> {
    if SYNTHETIC_FILTER_KEYS.contains(&field) || is_known_field(field, entity_def) {
        Ok(())
    } else {
        Err(Error::Validation(format!("Unknown filter field: {field}")))
    }
}

fn is_known_field(field: &str, entity_def: &EntityDefinition) -> bool {
    SYSTEM_FIELDS.contains(&field) || entity_def.get_field(field).is_some()
}

/// Double-quote a Postgres identifier, escaping embedded quotes. Only reached
/// for identifiers already confirmed against the allowlist (defense in depth).
fn quote_ident(field: &str) -> String {
    format!("\"{}\"", field.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_core::entity_definition::definition::EntityDefinition;

    fn empty_def() -> EntityDefinition {
        EntityDefinition::default()
    }

    #[test]
    fn system_fields_are_allowed_and_quoted() {
        let def = empty_def();
        assert_eq!(validate_and_quote("uuid", &def).unwrap(), "\"uuid\"");
        assert_eq!(
            validate_and_quote("created_at", &def).unwrap(),
            "\"created_at\""
        );
        // Regression: entity_key is a registry-projected system column and must
        // be filterable/sortable even when absent from the entity definition.
        assert_eq!(
            validate_and_quote("entity_key", &def).unwrap(),
            "\"entity_key\""
        );
        assert!(validate_filter_key("entity_key", &def).is_ok());
    }

    #[test]
    fn injection_attempt_is_rejected() {
        let def = empty_def();
        assert!(validate_and_quote("uuid; DROP TABLE admin_users; --", &def).is_err());
        assert!(validate_and_quote("1=1", &def).is_err());
        assert!(validate_and_quote("name) OR (1=1", &def).is_err());
    }

    #[test]
    fn unknown_plain_field_is_rejected() {
        let def = empty_def();
        assert!(validate_and_quote("totally_unknown", &def).is_err());
    }

    #[test]
    fn synthetic_filter_keys_pass_filter_key_check() {
        let def = empty_def();
        assert!(validate_filter_key("path_prefix", &def).is_ok());
        assert!(validate_filter_key("path_equals", &def).is_ok());
        assert!(validate_filter_key("nope", &def).is_err());
    }
}
