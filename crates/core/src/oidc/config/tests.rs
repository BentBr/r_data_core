#![allow(clippy::expect_used)]

//! Weighted towards the refusals and the defaults, because those are what a
//! misconfiguration turns into: a system that lets in people it should not,
//! and gives no sign of it.

use super::*;

fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn enabled() -> Vec<(&'static str, &'static str)> {
    vec![
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "r-data-core"),
    ]
}

fn config_from(pairs: &[(&str, &str)]) -> OidcConfig {
    OidcConfig::from_map(&env(pairs))
        .expect("valid configuration")
        .expect("OIDC should be enabled")
}

// ── opt-in ──────────────────────────────────────────────────────────────────

#[test]
fn no_issuer_means_oidc_is_off() {
    assert_eq!(
        OidcConfig::from_map(&env(&[])).expect("no error"),
        None,
        "OIDC must be opt-in; an instance that never configured it must be untouched"
    );
}

#[test]
fn an_empty_issuer_also_means_off() {
    // `RDC_OIDC_ISSUER=` in a compose file is how people disable things.
    assert_eq!(
        OidcConfig::from_map(&env(&[("RDC_OIDC_ISSUER", "")])).expect("no error"),
        None
    );
}

// ── refusals ────────────────────────────────────────────────────────────────

#[test]
fn an_issuer_without_an_audience_is_rejected() {
    // Without an audience check, any application sharing the IdP can mint a
    // token this instance will accept.
    let err = OidcConfig::from_map(&env(&[("RDC_OIDC_ISSUER", "https://auth.example.com")]))
        .expect_err("audience is mandatory");
    assert_eq!(err, OidcConfigError::MissingAudience);
}

#[test]
fn a_malformed_role_map_entry_is_rejected_not_skipped() {
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_ROLE_MAP", "rdc-admins:admin,broken"));
    let err = OidcConfig::from_map(&env(&pairs)).expect_err("malformed entry");
    assert_eq!(
        err,
        OidcConfigError::MalformedRoleMap("broken".to_string()),
        "skipping it would leave a map that grants nothing, with no clue why"
    );
}

#[test]
fn a_role_map_entry_with_an_empty_side_is_rejected() {
    for bad in ["group:", ":role", ":"] {
        let mut pairs = enabled();
        pairs.push(("RDC_OIDC_ROLE_MAP", bad));
        assert!(
            OidcConfig::from_map(&env(&pairs)).is_err(),
            "'{bad}' should be refused"
        );
    }
}

#[test]
fn a_non_numeric_jwks_ttl_is_rejected() {
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_JWKS_TTL_SECS", "hourly"));
    let err = OidcConfig::from_map(&env(&pairs)).expect_err("ttl must be numeric");
    assert!(matches!(
        err,
        OidcConfigError::InvalidNumber("RDC_OIDC_JWKS_TTL_SECS", _)
    ));
}

// ── the defaults that matter ────────────────────────────────────────────────

#[test]
fn no_default_role_means_reject_not_admit_with_nothing() {
    // Admitting an unmapped user creates a real account with a real session
    // that happens to be able to do nothing today — and that changes the
    // moment someone grants a broad default.
    assert_eq!(
        config_from(&enabled()).default_role,
        None,
        "the absence of a default must mean deny"
    );
}

#[test]
fn an_empty_default_role_is_treated_as_absent() {
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_DEFAULT_ROLE", ""));
    assert_eq!(config_from(&pairs).default_role, None);
}

#[test]
fn email_linking_is_off_unless_asked_for() {
    // Linking by address lets anyone who can register that address at a
    // trusted issuer inherit the matching local account.
    assert!(!config_from(&enabled()).link_by_email);
}

#[test]
fn email_linking_is_only_enabled_by_an_explicit_true() {
    for value in ["false", "0", "no", "yes", ""] {
        let mut pairs = enabled();
        pairs.push(("RDC_OIDC_LINK_BY_EMAIL", value));
        assert!(
            !config_from(&pairs).link_by_email,
            "'{value}' must not enable email linking"
        );
    }
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_LINK_BY_EMAIL", "true"));
    assert!(config_from(&pairs).link_by_email);
}

#[test]
fn the_roles_claim_defaults_to_groups() {
    assert_eq!(config_from(&enabled()).roles_claim, "groups");
}

#[test]
fn the_jwks_ttl_defaults_to_an_hour() {
    assert_eq!(config_from(&enabled()).jwks_ttl, Duration::from_secs(3600));
}

// ── parsing ─────────────────────────────────────────────────────────────────

#[test]
fn a_role_map_is_parsed() {
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_ROLE_MAP", "rdc-admins:admin,rdc-ops:editor"));
    let config = config_from(&pairs);
    assert_eq!(
        config.role_map.get("rdc-admins"),
        Some(&"admin".to_string())
    );
    assert_eq!(config.role_map.get("rdc-ops"), Some(&"editor".to_string()));
}

#[test]
fn role_map_whitespace_is_tolerated() {
    // Multi-line environment values in compose files pick up spaces.
    let mut pairs = enabled();
    pairs.push(("RDC_OIDC_ROLE_MAP", " rdc-admins : admin , rdc-ops:editor "));
    let config = config_from(&pairs);
    assert_eq!(
        config.role_map.get("rdc-admins"),
        Some(&"admin".to_string())
    );
    assert_eq!(config.role_map.get("rdc-ops"), Some(&"editor".to_string()));
}

#[test]
fn an_absent_role_map_is_empty_rather_than_an_error() {
    // Legitimate when every user gets the default role.
    assert!(config_from(&enabled()).role_map.is_empty());
}
