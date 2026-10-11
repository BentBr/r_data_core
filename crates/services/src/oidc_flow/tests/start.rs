#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Starting a sign-in: PKCE is present and correctly derived, each sign-in gets
//! fresh values, and the landing URL cannot carry the browser off this origin.

use super::*;

#[tokio::test]
async fn start_sends_the_browser_to_the_provider_with_pkce_and_state() {
    let server = provider().await;
    let redirect = flow(&server, &[]).start(None).await.expect("start");
    let q = query_of(&redirect.url);

    assert!(redirect
        .url
        .starts_with(&format!("{}/authorize", server.uri())));
    assert_eq!(q.get("response_type").map(String::as_str), Some("code"));
    assert_eq!(q.get("client_id").map(String::as_str), Some("rdc"));
    assert_eq!(
        q.get("code_challenge_method").map(String::as_str),
        Some("S256"),
        "PKCE is mandatory, and plain is not PKCE"
    );
    assert!(q.contains_key("code_challenge"));
    assert!(q.contains_key("nonce"));
    assert_eq!(
        q.get("state").map(String::as_str),
        Some(redirect.state.as_str())
    );
}

#[tokio::test]
async fn the_challenge_is_the_hash_of_the_verifier_not_the_verifier() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow.start(None).await.expect("start");
    let q = query_of(&redirect.url);
    let challenge = q.get("code_challenge").expect("challenge");

    // Recover the verifier the flow stored and re-derive the challenge. If
    // the two ever diverge the provider rejects every exchange, and if the
    // challenge *were* the verifier, PKCE would protect nothing.
    let pending: PendingLogin = flow
        .cache
        .get(&format!("{STATE_PREFIX}{}", redirect.state))
        .await
        .expect("cache")
        .expect("stored");

    assert_eq!(
        *challenge,
        URL_SAFE_NO_PAD.encode(Sha256::digest(pending.verifier.as_bytes()))
    );
    assert_ne!(*challenge, pending.verifier);
}

#[tokio::test]
async fn every_sign_in_gets_a_fresh_state_and_verifier() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let first = flow.start(None).await.expect("first");
    let second = flow.start(None).await.expect("second");

    assert_ne!(first.state, second.state);
    assert_ne!(
        query_of(&first.url).get("code_challenge"),
        query_of(&second.url).get("code_challenge")
    );
}

#[tokio::test]
async fn the_flow_is_refused_when_no_client_is_configured() {
    let server = MockServer::start().await;
    let config = OidcConfig::from_map(
        &[
            ("RDC_OIDC_ISSUER".to_string(), server.uri()),
            ("RDC_OIDC_AUDIENCE".to_string(), "r-data-core".to_string()),
        ]
        .into_iter()
        .collect(),
    )
    .expect("valid")
    .expect("enabled");

    let cache = Arc::new(CacheManager::new(CacheConfig::default()));
    let runtime = Arc::new(OidcRuntime::new(
        config,
        Arc::new(StaticKeySource::new(JwkSet::default())),
        Arc::new(OidcProvisioningService::new(
            Arc::new(FakeIdentities::default()),
            Arc::new(FakeUsers::default()),
            Arc::new(crate::oidc_test_fakes::FakeRoles {
                roles: vec![],
                user_roles: vec![],
            }),
        )),
        Arc::clone(&cache),
    ));
    let flow = OidcFlow::new(runtime, cache, None).expect("flow");

    assert!(!flow.is_configured());
    assert!(matches!(
        flow.start(None).await,
        Err(FlowError::NotConfigured)
    ));
}

#[tokio::test]
async fn the_landing_url_is_resolved_against_the_interface_origin() {
    // The callback runs on the API's origin. When the admin interface is
    // served elsewhere — as it is in the local compose stack — a bare path
    // would send the browser to a host that serves no interface, and the
    // sign-in would appear to succeed and then land on nothing.
    let server = provider().await;
    let base = flow_with_frontend(&server, Some("https://admin.example.com"));

    assert_eq!(
        base.landing_path(None),
        "https://admin.example.com/dashboard"
    );
    assert_eq!(
        base.landing_path(Some("/workflows/42")),
        "https://admin.example.com/workflows/42"
    );
}

#[tokio::test]
async fn a_hostile_return_to_cannot_escape_even_with_a_base_configured() {
    let server = provider().await;
    let base = flow_with_frontend(&server, Some("https://admin.example.com"));

    for escape in [
        "https://evil.example.com",
        "//evil.example.com",
        "/\\evil.example.com",
    ] {
        assert_eq!(
            base.landing_path(Some(escape)),
            "https://admin.example.com/dashboard",
            "{escape:?} must be discarded before the base is applied"
        );
    }
}

#[tokio::test]
async fn a_bare_path_is_kept_when_the_interface_shares_the_origin() {
    let server = provider().await;
    assert_eq!(flow(&server, &[]).landing_path(None), "/dashboard");
}
