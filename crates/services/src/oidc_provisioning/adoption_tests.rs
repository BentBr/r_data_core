#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Which account a first sign-in may become, and which it may not.

use std::sync::Arc;

use uuid::Uuid;

use super::*;
use crate::oidc_test_fakes::{
    claims, config, service, service_with_user_roles, super_admin, user, FakeIdentities, FakeUsers,
};

#[tokio::test]
async fn an_unverified_email_is_not_written_to_the_new_account() {
    // The address is attacker-chosen until the provider vouches for it. A
    // provisioned account that displays it would lend it credibility.
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());

    let (resolved, _) = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(
            &claims("mallory", &["rdc-ops"], false),
            &config(None, false),
        )
        .await
        .expect("provisioned");

    assert_eq!(resolved.email, "mallory@sso.invalid");
    assert_eq!(resolved.username, "mallory");
}

#[tokio::test]
async fn a_verified_email_is_kept() {
    let (resolved, _) = service(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    )
    .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
    .await
    .expect("provisioned");

    assert_eq!(resolved.email, "ada@example.com");
}

#[tokio::test]
async fn an_address_already_held_locally_is_refused_not_a_server_error() {
    // Linking is off, so adoption is not on the table; creating a second
    // account with the same address is a unique violation. That is a
    // refusal of this sign-in, not an outage.
    let identities = Arc::new(FakeIdentities::default());
    let users = Arc::new(FakeUsers::default());
    users
        .taken_emails
        .lock()
        .expect("lock")
        .push("ada@example.com".to_string());

    let err = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, false))
        .await
        .expect_err("refused");

    assert!(matches!(err, Error::Auth(_)), "got {err:?}");
    assert!(
        identities.links.lock().expect("lock").is_empty(),
        "a refused sign-in must leave no link behind"
    );
}

#[tokio::test]
async fn a_super_admin_account_is_never_adopted_by_email() {
    // The provider's say-so over an address must not be able to hand over
    // the whole instance. Super-admin is granted inside RDataCore, to a
    // named person; it is never reachable from a claim.
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("root@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![super_admin(local, "root")]));

    let err = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("root", &["rdc-ops"], true), &config(None, true))
        .await
        .expect_err("refused");

    assert!(matches!(err, Error::Auth(_)), "got {err:?}");
    assert_eq!(users.created_count(), 0, "and no shadow account either");
    assert!(identities.links.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn an_account_super_admin_through_a_role_is_never_adopted_by_email() {
    // The flag is not the only way to be super-admin: `admin_jwt` honours a
    // role with `super_admin` too, and an adopted account's first password
    // session would carry it.
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ops@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ops", true)]));
    let mut root_role = Role::new("root".to_string());
    root_role.super_admin = true;

    let err = service_with_user_roles(
        Arc::clone(&identities),
        Arc::clone(&users),
        vec![(local, root_role)],
    )
    .resolve_or_provision(&claims("ops", &["rdc-ops"], true), &config(None, true))
    .await
    .expect_err("refused");

    assert!(matches!(err, Error::Auth(_)), "got {err:?}");
    assert_eq!(users.created_count(), 0, "and no shadow account either");
    assert!(identities.links.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn an_ordinary_account_is_still_adoptable() {
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let (resolved, _) = service(identities, users)
        .resolve_or_provision(&claims("ada", &["rdc-ops"], true), &config(None, true))
        .await
        .expect("adopted");
    assert_eq!(resolved.uuid, local);
}

#[tokio::test]
async fn an_unverified_claim_matching_a_local_email_is_refused_and_creates_nothing() {
    // Review focus: the claimed address equals a local user's, but the
    // provider does not vouch for it. No adoption, and no row created.
    let local = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .local_by_email
        .lock()
        .expect("lock")
        .insert("ada@example.com".to_string(), local);
    let users = Arc::new(FakeUsers::with(vec![user(local, "ada", true)]));

    let err = service(Arc::clone(&identities), Arc::clone(&users))
        .resolve_or_provision(&claims("ada", &["rdc-ops"], false), &config(None, true))
        .await
        .expect_err("refused");

    assert!(matches!(err, Error::Auth(_)), "got {err:?}");
    assert_eq!(users.created_count(), 0);
    assert!(identities.links.lock().expect("lock").is_empty());
}
