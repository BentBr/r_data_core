#![allow(clippy::expect_used)]

//! The two non-negotiable rules get the most tests, because both are
//! deliberately awkward and both would be tempting to soften.

use std::collections::HashMap;

use super::*;

fn role(name: &str) -> Role {
    Role::new(name.to_string())
}

fn super_admin_role(name: &str) -> Role {
    let mut role = Role::new(name.to_string());
    role.super_admin = true;
    role
}

fn claims_with(groups: &[&str]) -> OidcClaims {
    OidcClaims {
        issuer: "https://auth.example.com".to_string(),
        subject: "user-1".to_string(),
        email: Some("ada@example.com".to_string()),
        email_verified: true,
        name: Some("Ada".to_string()),
        groups: groups.iter().map(|g| (*g).to_string()).collect(),
        raw: HashMap::new(),
    }
}

fn config_with(map: &[(&str, &str)], default_role: Option<&str>) -> OidcConfig {
    let mut pairs = vec![
        (
            "RDC_OIDC_ISSUER".to_string(),
            "https://auth.example.com".to_string(),
        ),
        ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
    ];
    if !map.is_empty() {
        let joined = map
            .iter()
            .map(|(g, r)| format!("{g}:{r}"))
            .collect::<Vec<_>>()
            .join(",");
        pairs.push(("RDC_OIDC_ROLE_MAP".to_string(), joined));
    }
    if let Some(default) = default_role {
        pairs.push(("RDC_OIDC_DEFAULT_ROLE".to_string(), default.to_string()));
    }
    OidcConfig::from_map(&pairs.into_iter().collect())
        .expect("valid")
        .expect("enabled")
}

// ── the mapping itself ──────────────────────────────────────────────────────

#[test]
fn a_matching_group_yields_its_role() {
    let roles = map_roles(
        &claims_with(&["rdc-admins"]),
        &config_with(&[("rdc-admins", "admin")], None),
        &[role("admin"), role("editor")],
    )
    .expect("should map");

    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "admin");
}

#[test]
fn several_groups_yield_several_roles() {
    let roles = map_roles(
        &claims_with(&["rdc-admins", "rdc-ops"]),
        &config_with(&[("rdc-admins", "admin"), ("rdc-ops", "editor")], None),
        &[role("admin"), role("editor")],
    )
    .expect("should map");
    assert_eq!(roles.len(), 2);
}

#[test]
fn two_groups_mapping_to_one_role_do_not_duplicate_it() {
    // Otherwise the audit trail shows a user holding the same role twice,
    // which reads as though something went wrong.
    let roles = map_roles(
        &claims_with(&["group-a", "group-b"]),
        &config_with(&[("group-a", "editor"), ("group-b", "editor")], None),
        &[role("editor")],
    )
    .expect("should map");
    assert_eq!(roles.len(), 1);
}

#[test]
fn groups_that_map_to_nothing_are_ignored() {
    let roles = map_roles(
        &claims_with(&["rdc-admins", "some-unrelated-team"]),
        &config_with(&[("rdc-admins", "admin")], None),
        &[role("admin")],
    )
    .expect("should map");
    assert_eq!(roles.len(), 1);
}

// ── deny by default ─────────────────────────────────────────────────────────

#[test]
fn an_unmapped_user_is_rejected_when_no_default_is_configured() {
    let err = map_roles(
        &claims_with(&["some-unrelated-team"]),
        &config_with(&[("rdc-admins", "admin")], None),
        &[role("admin")],
    )
    .expect_err("an unmapped user must be rejected, not admitted empty-handed");
    assert_eq!(err, MappingError::NoMappedRole);
}

#[test]
fn a_user_with_no_groups_at_all_is_rejected() {
    let err = map_roles(
        &claims_with(&[]),
        &config_with(&[("rdc-admins", "admin")], None),
        &[role("admin")],
    )
    .expect_err("no groups means no roles means no access");
    assert_eq!(err, MappingError::NoMappedRole);
}

#[test]
fn an_unmapped_user_gets_the_default_role_when_one_is_configured() {
    let roles = map_roles(
        &claims_with(&["some-unrelated-team"]),
        &config_with(&[("rdc-admins", "admin")], Some("viewer")),
        &[role("admin"), role("viewer")],
    )
    .expect("the default should apply");
    assert_eq!(roles[0].name, "viewer");
}

#[test]
fn a_default_naming_a_nonexistent_role_still_denies() {
    // A typo in the default must not become a way in.
    let err = map_roles(
        &claims_with(&["some-unrelated-team"]),
        &config_with(&[], Some("no-such-role")),
        &[role("admin")],
    )
    .expect_err("a broken default must deny");
    assert_eq!(err, MappingError::NoMappedRole);
}

#[test]
fn a_mapping_naming_a_nonexistent_role_falls_through_to_deny() {
    let err = map_roles(
        &claims_with(&["rdc-admins"]),
        &config_with(&[("rdc-admins", "no-such-role")], None),
        &[role("admin")],
    )
    .expect_err("a mapping to nothing must not admit");
    assert_eq!(err, MappingError::NoMappedRole);
}

// ── super-admin is never grantable by claim ─────────────────────────────────

#[test]
fn a_group_mapped_to_a_super_admin_role_does_not_grant_super_admin() {
    // The identity provider's group list is often editable by people who do
    // not administer this system. A string match must not hand over the
    // instance.
    let roles = map_roles(
        &claims_with(&["rdc-admins", "rdc-ops"]),
        &config_with(&[("rdc-admins", "superuser"), ("rdc-ops", "editor")], None),
        &[super_admin_role("superuser"), role("editor")],
    )
    .expect("the non-super-admin role should still apply");

    assert!(
        roles.iter().all(|r| !r.super_admin),
        "claim mapping must never yield a super-admin role: {roles:?}"
    );
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].name, "editor");
}

#[test]
fn a_super_admin_role_as_the_only_match_denies_rather_than_admitting_nothing() {
    // Dropping the only match leaves an empty role set. Admitting that would
    // create a session with no permissions rather than refusing entry.
    let err = map_roles(
        &claims_with(&["rdc-admins"]),
        &config_with(&[("rdc-admins", "superuser")], None),
        &[super_admin_role("superuser")],
    )
    .expect_err("nothing left to grant means no access");
    assert_eq!(err, MappingError::NoMappedRole);
}

#[test]
fn a_super_admin_default_role_is_also_refused() {
    // The same reasoning applies to the default: it is configuration, but so
    // is the role map, and neither should be able to confer super-admin.
    let err = map_roles(
        &claims_with(&["unmapped"]),
        &config_with(&[], Some("superuser")),
        &[super_admin_role("superuser")],
    )
    .expect_err("a super-admin default must not confer super-admin");
    assert_eq!(err, MappingError::NoMappedRole);
}
