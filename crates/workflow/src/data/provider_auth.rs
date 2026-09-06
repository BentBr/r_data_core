#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Validation for the top-level `provider_auth` block.

use r_data_core_core::error::{Error, Result};
use r_data_core_core::validation::constraints::MIN_PRE_SHARED_KEY_LEN;

use crate::data::adapters::auth::AuthConfig;

/// Validate the top-level `provider_auth` block — the value the public workflow
/// endpoints authenticate against.
///
/// Distinct from `dsl::to::validate_auth_config`, which polices what a workflow
/// *sends* to third parties. This one guards the door into us.
///
/// A missing or unparseable block is not an error: the endpoint already treats
/// an unreadable `provider_auth` as "none configured", and failing here would
/// break a workflow over an unrelated malformed edit.
///
/// # Errors
/// Returns [`Error::Validation`] when a pre-shared key is empty or shorter than
/// [`MIN_PRE_SHARED_KEY_LEN`].
pub fn validate_provider_auth_config(config: &serde_json::Value) -> Result<()> {
    let Some(raw) = config.get("provider_auth") else {
        return Ok(());
    };
    let Ok(auth) = serde_json::from_value::<AuthConfig>(raw.clone()) else {
        return Ok(());
    };

    if let AuthConfig::PreSharedKey { key, .. } = auth {
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::Validation(
                "provider_auth.pre_shared_key.key must not be empty".to_string(),
            ));
        }
        if key.chars().count() < MIN_PRE_SHARED_KEY_LEN {
            return Err(Error::Validation(format!(
                "provider_auth.pre_shared_key.key must be at least \
                 {MIN_PRE_SHARED_KEY_LEN} characters - it is the only thing \
                 guarding the public endpoint"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_provider_auth_config;
    use serde_json::json;

    fn psk(key: &str) -> serde_json::Value {
        json!({
            "provider_auth": {
                "type": "pre_shared_key",
                "key": key,
                "location": "header",
                "field_name": "X-Pre-Shared-Key"
            }
        })
    }

    #[test]
    fn no_provider_auth_is_fine() {
        assert!(validate_provider_auth_config(&json!({})).is_ok());
    }

    #[test]
    fn a_long_enough_key_is_accepted() {
        assert!(validate_provider_auth_config(&psk(&"a".repeat(32))).is_ok());
    }

    /// The point of this module: this is the value the public endpoint checks.
    #[test]
    fn a_short_key_is_rejected() {
        let err = validate_provider_auth_config(&psk("short")).expect_err("must reject");
        assert!(
            err.to_string().contains("32"),
            "error must state the length: {err}"
        );
    }

    #[test]
    fn an_empty_key_is_rejected() {
        assert!(validate_provider_auth_config(&psk("")).is_err());
        assert!(validate_provider_auth_config(&psk("   ")).is_err());
    }

    #[test]
    fn whitespace_does_not_count_toward_the_length() {
        assert!(validate_provider_auth_config(&psk(&format!("  {}  ", "a".repeat(30)))).is_err());
    }

    #[test]
    fn other_provider_auth_kinds_are_untouched() {
        let jwt = json!({ "provider_auth": { "type": "entity_jwt" } });
        assert!(validate_provider_auth_config(&jwt).is_ok());
    }

    /// The endpoint already treats an unparseable block as "none configured",
    /// so rejecting it here would break a workflow over an unrelated bad edit.
    #[test]
    fn malformed_provider_auth_is_not_rejected_here() {
        assert!(validate_provider_auth_config(&json!({ "provider_auth": "nonsense" })).is_ok());
    }
}
