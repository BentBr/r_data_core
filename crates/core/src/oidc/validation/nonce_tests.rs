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
