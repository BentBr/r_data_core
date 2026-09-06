#![allow(clippy::expect_used)]

use super::*;

// ── permission matching ─────────────────────────────────────────────────────

#[test]
fn an_exact_permission_matches() {
    let perms = Permissions {
        is_super_admin: false,
        entries: vec!["workflows:read".to_string()],
    };
    assert!(perms.allows("workflows:read"));
    assert!(!perms.allows("workflows:write"));
}

#[test]
fn super_admin_allows_everything() {
    let perms = Permissions {
        is_super_admin: true,
        entries: Vec::new(),
    };
    assert!(perms.allows("workflows:write"));
    assert!(perms.allows("anything:at:all"));
}

#[test]
fn a_path_scoped_permission_still_grants_visibility() {
    // entities:/customers:read means "can read SOME entities". Hiding the
    // query tool from this caller would be wrong: they can use it, just not
    // everywhere, and RDataCore enforces the path on the actual call.
    let perms = Permissions {
        is_super_admin: false,
        entries: vec!["entities:/customers:read".to_string()],
    };
    assert!(perms.allows("entities:read"));
    assert!(!perms.allows("entities:write"));
}

#[test]
fn a_path_scoped_permission_does_not_grant_another_action() {
    let perms = Permissions {
        is_super_admin: false,
        entries: vec!["workflows:/imports:read".to_string()],
    };
    assert!(perms.allows("workflows:read"));
    assert!(!perms.allows("workflows:write"));
}

#[test]
fn a_permission_in_another_namespace_does_not_match() {
    let perms = Permissions {
        is_super_admin: false,
        entries: vec!["entities:read".to_string()],
    };
    assert!(!perms.allows("workflows:read"));
}

#[test]
fn a_malformed_requirement_matches_nothing() {
    // A requirement without a namespace is a programming error, and silently
    // matching would be the dangerous direction to fail in.
    let perms = Permissions {
        is_super_admin: false,
        entries: vec!["workflows:read".to_string()],
    };
    assert!(!perms.allows("workflows"));
}

#[test]
fn no_permissions_allows_nothing() {
    assert!(!Permissions::default().allows("workflows:read"));
}

// ── the API key backend ─────────────────────────────────────────────────────

#[tokio::test]
async fn attaches_the_credential_header() {
    let backend = ApiKeyBackend::new("secret-value".to_string());
    let headers = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("headers");

    assert!(
        headers
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "secret-value"),
        "expected the credential header RDataCore reads, got {headers:?}"
    );
}

#[tokio::test]
async fn attaches_nothing_else() {
    // An extra header here would be sent on every request; keep the surface
    // exactly one header wide.
    let backend = ApiKeyBackend::new("secret-value".to_string());
    let headers = backend
        .outbound_headers(&CallerContext::default())
        .await
        .expect("headers");
    assert_eq!(headers.len(), 1, "got {headers:?}");
}
