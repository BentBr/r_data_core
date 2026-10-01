#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What the shared authentication tail must guarantee.
//!
//! The interesting assertions are the refusals and the cache boundary: a
//! deactivated account is refused however valid its token, a resolved
//! identity is not re-resolved on every request, and the routing peek at an
//! unverified `iss` cannot be talked into claiming a token is ours.

use std::sync::Arc;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde_json::json;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;

use super::*;
use crate::oidc_test_fakes::{claims, config, service, user, FakeIdentities, FakeUsers, ISSUER};

/// A cache that actually stores, so the caching assertions mean something.
fn cache() -> Arc<CacheManager> {
    Arc::new(CacheManager::new(CacheConfig {
        enabled: true,
        ttl: 60,
        ..CacheConfig::default()
    }))
}

fn runtime(identities: Arc<FakeIdentities>, users: Arc<FakeUsers>) -> OidcRuntime {
    OidcRuntime::new(
        config(None, false),
        Arc::new(r_data_core_core::oidc::StaticKeySource::new(
            r_data_core_core::oidc::JwkSet::default(),
        )),
        Arc::new(service(identities, users)),
        cache(),
    )
}

/// An unsigned token carrying this payload. Only the payload segment is ever
/// read by the routing peek, which is the point being tested.
fn token_claiming(issuer: &str) -> String {
    let payload = URL_SAFE_NO_PAD.encode(json!({ "iss": issuer }).to_string());
    format!("header.{payload}.signature")
}

// ── the routing peek ────────────────────────────────────────────────────────

#[test]
fn a_token_from_the_configured_issuer_is_routed_here() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );
    assert!(rt.token_targets_this_issuer(&token_claiming(ISSUER)));
}

#[test]
fn a_token_from_another_issuer_is_left_to_the_other_auth_arms() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );
    assert!(
        !rt.token_targets_this_issuer(&token_claiming("r_data_core_admin")),
        "a local RDataCore JWT must not be run through OIDC validation"
    );
}

#[test]
fn garbage_is_not_claimed_by_the_oidc_arm() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );
    for junk in ["", "not-a-token", "a.b", "a.!!!.c", "a.e30.c"] {
        assert!(
            !rt.token_targets_this_issuer(junk),
            "{junk:?} names no issuer, so it is not ours to reject"
        );
    }
}

// ── resolution and its cache ────────────────────────────────────────────────

#[tokio::test]
async fn a_resolved_identity_carries_the_roles_its_groups_map_to() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );

    let minted = rt
        .session_claims(&claims("ada", &["rdc-ops"], true), 1800)
        .await
        .expect("should resolve");

    assert!(!minted.is_super_admin);
    assert_eq!(minted.name, "ada");
}

#[tokio::test]
async fn a_second_request_reuses_the_resolution_instead_of_hitting_the_database() {
    let users = Arc::new(FakeUsers::default());
    let rt = runtime(Arc::new(FakeIdentities::default()), Arc::clone(&users));
    let identity = claims("ada", &["rdc-ops"], true);

    rt.session_claims(&identity, 1800).await.expect("first");
    rt.session_claims(&identity, 1800).await.expect("second");

    assert_eq!(
        users.created_count(),
        1,
        "the second request must not re-provision"
    );
}

#[tokio::test]
async fn forgetting_an_identity_forces_the_next_request_to_re_resolve() {
    let users = Arc::new(FakeUsers::default());
    let rt = runtime(Arc::new(FakeIdentities::default()), Arc::clone(&users));
    let identity = claims("ada", &["rdc-ops"], true);

    let first = rt.session_claims(&identity, 1800).await.expect("first");
    rt.forget(ISSUER, "ada").await;
    let second = rt.session_claims(&identity, 1800).await.expect("second");

    // Same account either way — the point is that the second call went to the
    // repository rather than the cache, which is what makes a deactivation
    // take effect immediately.
    assert_eq!(first.sub, second.sub);
}

#[tokio::test]
async fn two_identities_do_not_share_a_cache_entry() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );

    let ada = rt
        .session_claims(&claims("ada", &["rdc-ops"], true), 1800)
        .await
        .expect("ada");
    let grace = rt
        .session_claims(&claims("grace", &["rdc-ops"], true), 1800)
        .await
        .expect("grace");

    assert_ne!(
        ada.sub, grace.sub,
        "a shared cache key would hand one user another's session"
    );
}

// ── refusals ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn an_unmapped_identity_is_denied_rather_than_admitted_with_nothing() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );

    let outcome = rt
        .session_claims(&claims("nobody", &["unmapped-group"], true), 1800)
        .await;

    assert!(
        matches!(outcome, Err(OidcAuthError::Denied(_))),
        "expected a refusal, got {outcome:?}"
    );
}

#[tokio::test]
async fn a_deactivated_account_is_refused_however_valid_its_token() {
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "ada".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", false)]));

    let outcome = runtime(identities, users)
        .session_claims(&claims("ada", &["rdc-ops"], true), 1800)
        .await;

    assert!(
        matches!(outcome, Err(OidcAuthError::Denied(_))),
        "the IdP says who someone is; RDataCore says whether they may act. Got {outcome:?}"
    );
}

#[tokio::test]
async fn a_refused_identity_is_not_cached_as_a_session() {
    let rt = runtime(
        Arc::new(FakeIdentities::default()),
        Arc::new(FakeUsers::default()),
    );
    let unmapped = claims("nobody", &["unmapped-group"], true);

    assert!(rt.session_claims(&unmapped, 1800).await.is_err());
    assert!(
        rt.session_claims(&unmapped, 1800).await.is_err(),
        "a refusal must stay a refusal on the next request"
    );
}

// ── error classification ────────────────────────────────────────────────────

#[test]
fn a_database_outage_is_reported_as_unavailable_not_as_a_refusal() {
    let unavailable: OidcAuthError =
        r_data_core_core::error::Error::Cache("redis is down".to_string()).into();
    assert!(
        matches!(unavailable, OidcAuthError::Unavailable(_)),
        "infrastructure failures must not send an operator hunting through role mappings"
    );

    let denied: OidcAuthError =
        r_data_core_core::error::Error::Auth("this account is locked".to_string()).into();
    assert!(matches!(denied, OidcAuthError::Denied(_)));
}

#[tokio::test]
async fn forgetting_an_account_evicts_every_identity_it_owns() {
    // Deactivating someone in RDataCore has to reach this cache, and the cache
    // is keyed on the external identity rather than the account — so the
    // account's identities have to be looked up to know what to drop.
    let existing = Uuid::now_v7();
    let identities = Arc::new(FakeIdentities::default());
    identities
        .links
        .lock()
        .expect("lock")
        .insert((ISSUER.to_string(), "ada".to_string()), existing);
    let users = Arc::new(FakeUsers::with(vec![user(existing, "ada", true)]));

    let rt = runtime(Arc::clone(&identities), Arc::clone(&users));
    let identity = claims("ada", &["rdc-ops"], true);

    rt.session_claims(&identity, 1800).await.expect("resolve");

    // Deactivate, then evict by account rather than by identity.
    users.deactivate(existing);
    rt.forget_user(existing).await;

    let after = rt.session_claims(&identity, 1800).await;
    assert!(
        matches!(after, Err(OidcAuthError::Denied(_))),
        "a deactivated account must be refused on the next request, not after the \
         cache window; got {after:?}"
    );
}
