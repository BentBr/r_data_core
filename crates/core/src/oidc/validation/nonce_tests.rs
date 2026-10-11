#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The claims that bind a token to one sign-in attempt and one client.

use serde_json::json;

use super::tests::{claims, config, key_set, keypair, sign, NOW};
use super::*;

#[test]
fn a_matching_nonce_is_accepted() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "nonce": "n-1" })), &key);
    validate_token(&token, &key_set(jwk), &config(), NOW, Some("n-1")).expect("should validate");
}

#[test]
fn a_different_nonce_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "nonce": "n-2" })), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW, Some("n-1")).expect_err("mismatch"),
        ValidationError::NonceMismatch
    );
}

#[test]
fn a_missing_nonce_is_rejected_when_one_was_sent() {
    // An attacker injecting a token obtained elsewhere has no nonce to put in
    // it. Absence must count as mismatch, not as "nothing to check".
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({})), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW, Some("n-1")).expect_err("absent"),
        ValidationError::NonceMismatch
    );
}

#[test]
fn a_nonce_is_ignored_when_none_was_sent() {
    // The bearer path and the machine exchange never sent one.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "nonce": "whatever" })), &key);
    validate_token(&token, &key_set(jwk), &config(), NOW, None).expect("should validate");
}

// ── azp ─────────────────────────────────────────────────────────────────────

fn config_with_client() -> OidcConfig {
    OidcConfig::from_map(
        &[
            ("RDC_OIDC_ISSUER", super::tests::ISSUER),
            ("RDC_OIDC_AUDIENCE", super::tests::AUDIENCE),
            ("RDC_OIDC_CLIENT_ID", "rdc"),
            ("RDC_OIDC_REDIRECT_URI", "https://rdc.example.com/cb"),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect(),
    )
    .expect("valid")
    .expect("enabled")
}

#[test]
fn a_multi_audience_token_must_name_this_client_as_authorized_party() {
    // OIDC Core 3.1.3.7: with several audiences, `azp` says which client the
    // token was really issued to. A token for another client that happens to
    // list this audience is not a token for this client.
    let (key, jwk) = keypair();
    let token = sign(
        &claims(json!({ "aud": [super::tests::AUDIENCE, "other"], "azp": "other" })),
        &key,
    );
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config_with_client(), NOW, None)
            .expect_err("wrong azp"),
        ValidationError::AzpMismatch
    );
}

#[test]
fn a_multi_audience_token_without_azp_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(
        &claims(json!({ "aud": [super::tests::AUDIENCE, "other"] })),
        &key,
    );
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config_with_client(), NOW, None)
            .expect_err("missing azp"),
        ValidationError::AzpMismatch
    );
}

#[test]
fn a_multi_audience_token_naming_this_client_is_accepted() {
    let (key, jwk) = keypair();
    let token = sign(
        &claims(json!({ "aud": [super::tests::AUDIENCE, "other"], "azp": "rdc" })),
        &key,
    );
    validate_token(&token, &key_set(jwk), &config_with_client(), NOW, None).expect("accepted");
}

#[test]
fn a_single_audience_token_needs_no_azp() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({})), &key);
    validate_token(&token, &key_set(jwk), &config_with_client(), NOW, None).expect("accepted");
}

// ── iat ─────────────────────────────────────────────────────────────────────

#[test]
fn a_token_issued_in_the_future_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "iat": NOW + 3600 })), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW, None).expect_err("future iat"),
        ValidationError::IssuedInFuture
    );
}

#[test]
fn an_iat_within_clock_skew_is_accepted() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "iat": NOW + 30 })), &key);
    validate_token(&token, &key_set(jwk), &config(), NOW, None).expect("within skew");
}
