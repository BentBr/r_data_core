#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The ordering guarantees provisioning must hold: a rejected identity leaves
//! no user row behind, and a deactivated account is refused however valid its
//! token. Fakes live in `crate::oidc_test_fakes`, shared with the runtime
//! tests so both assert against one set of behaviours.

use std::sync::Arc;

use uuid::Uuid;

use super::*;
use crate::oidc_test_fakes::{claims, config, service, user, FakeIdentities, FakeUsers, ISSUER};

// ── tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_first_sign_in_provisions_an_account() {
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let (user, roles) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(
            &claims("new-subject", &["rdc-ops"], true),
            &config(None, false),
        )
        .await
        .expect("should provision");

    assert_eq!(users.created_count(), 1);
    assert_eq!(roles[0].name, "editor");
    assert_eq!(user.username, "new-subject");
    assert!(
        identities
            .marked_sso
            .lock()
            .expect("lock")
            .contains(&user.uuid),
        "a provisioned account must be flagged, or the password paths stay open"
    );
}

#[tokio::test]
async fn a_known_identity_resolves_without_creating_anything() {
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "known".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("known", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("should resolve");

    assert_eq!(resolved.uuid, existing);
    assert_eq!(users.created_count(), 0);
}

#[tokio::test]
async fn a_rejected_identity_leaves_no_account_behind() {
    // Roles are resolved before anything is created, precisely so this holds.
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let result = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(
            &claims("unmapped", &["some-other-team"], true),
            &config(None, false),
        )
        .await;

    assert!(result.is_err(), "an unmapped identity must be refused");
    assert_eq!(
        users.created_count(),
        0,
        "a refused sign-in must not leave a user row"
    );
    assert!(identities.links.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn a_deactivated_account_is_refused_despite_a_valid_token() {
    // The provider says who someone is; RDataCore says whether they may act.
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "disabled".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", false)]));

    let err = service(identities, users)
        .resolve_or_provision(
            &claims("disabled", &["rdc-ops"], true),
            &config(None, false),
        )
        .await
        .expect_err("a deactivated account must not sign in");
    assert!(format!("{err}").contains("deactivated"), "{err}");
}

#[tokio::test]
async fn a_locked_account_is_refused() {
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "locked".to_string()), existing);
    let mut locked = user(existing, "ada", true);
    locked.locked_until = Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    let users = Arc::new(FakeUsers::with(vec![locked]));

    let err = service(identities, users)
        .resolve_or_provision(&claims("locked", &["rdc-ops"], true), &config(None, false))
        .await
        .expect_err("a locked account must not sign in");
    assert!(format!("{err}").contains("locked"), "{err}");
}

#[tokio::test]
async fn email_linking_is_refused_when_the_address_is_unverified() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], false), &config(None, true))
        .await
        .expect("should provision a new account instead");

    assert_ne!(
        resolved.uuid, local,
        "an unverified address must not adopt an existing account"
    );
    assert_eq!(users.created_count(), 1);
}

#[tokio::test]
async fn email_linking_is_refused_when_the_operator_has_not_enabled_it() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("should provision");

    assert_ne!(resolved.uuid, local, "linking is off by default");
}

#[tokio::test]
async fn email_linking_adopts_the_account_when_enabled_and_verified() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, true))
        .await
        .expect("should adopt");

    assert_eq!(resolved.uuid, local);
    assert_eq!(users.created_count(), 0, "no new account for an adoption");
    assert!(
        identities.marked_sso.lock().expect("lock").is_empty(),
        "an adopted local account keeps its password; only created ones are flagged"
    );
}

#[tokio::test]
async fn a_username_collision_does_not_fail_the_sign_in() {
    // Two people at different providers can share an email local-part.
    let taken = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::with(vec![user(taken, "ada", true)]));

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
        .await
        .expect("a collision must not block sign-in");

    assert_ne!(resolved.username, "ada");
    assert!(
        resolved.username.starts_with("ada-"),
        "got {}",
        resolved.username
    );
}

#[tokio::test]
async fn the_default_role_applies_to_an_unmapped_identity() {
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let (_, roles) = service(identities, users)
        .resolve_or_provision(
            &claims("unmapped", &["nothing-matches"], true),
            &config(Some("viewer"), false),
        )
        .await
        .expect("the default should apply");
    assert_eq!(roles[0].name, "viewer");
}

#[test]
fn a_display_name_is_split_into_first_and_last() {
    assert_eq!(
        split_name(Some("Ada Lovelace")),
        ("Ada".into(), "Lovelace".into())
    );
    assert_eq!(split_name(Some("Ada")), ("Ada".into(), String::new()));
    assert_eq!(
        split_name(Some("Ada King Lovelace")),
        ("Ada".into(), "King Lovelace".into())
    );
    assert_eq!(split_name(None), (String::new(), String::new()));
    assert_eq!(split_name(Some("   ")), (String::new(), String::new()));
}

#[test]
fn the_sentinel_password_is_long_and_never_the_same_twice() {
    let a = sentinel_password();
    let b = sentinel_password();
    assert_eq!(a.chars().count(), SENTINEL_PASSWORD_BYTES);
    assert_ne!(
        a, b,
        "a fixed sentinel would be shared across installations"
    );
}
