#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Almost all of these are rejections, which is the right balance: accepting a
//! good token is one path, and there are a dozen ways to be handed a bad one.

use std::collections::HashMap;

use base64::Engine as _;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;

use super::*;
use crate::oidc::keys::Jwk;

const ISSUER: &str = "https://auth.example.com";
const AUDIENCE: &str = "r-data-core";
const KID: &str = "test-key-1";
const NOW: i64 = 1_700_000_000;

/// A 2048-bit RSA key generated once for these tests.
///
/// Deterministic on purpose: generating one per run makes failures harder to
/// reproduce, and nothing here is secret.
fn keypair() -> (EncodingKey, Jwk) {
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    // A fixed seed keeps the key stable across runs.
    let mut rng = <rand_chacha::ChaCha8Rng as rand_core::SeedableRng>::seed_from_u64(42);
    let private = RsaPrivateKey::new(&mut rng, 2048).expect("generate an RSA key");
    let der = private.to_pkcs1_der().expect("encode the private key");
    let encoding = EncodingKey::from_rsa_der(der.as_bytes());

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let jwk = Jwk {
        kid: KID.to_string(),
        kty: "RSA".to_string(),
        alg: Some("RS256".to_string()),
        n: Some(b64.encode(private.n().to_bytes_be())),
        e: Some(b64.encode(private.e().to_bytes_be())),
        crv: None,
        x: None,
        y: None,
    };
    (encoding, jwk)
}

fn config() -> OidcConfig {
    OidcConfig::from_map(
        &[("RDC_OIDC_ISSUER", ISSUER), ("RDC_OIDC_AUDIENCE", AUDIENCE)]
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
    )
    .expect("valid")
    .expect("enabled")
}

fn claims(overrides: serde_json::Value) -> HashMap<String, serde_json::Value> {
    let mut base: HashMap<String, serde_json::Value> = serde_json::from_value(json!({
        "iss": ISSUER,
        "aud": AUDIENCE,
        "sub": "user-1",
        "exp": NOW + 3600,
        "iat": NOW,
        "email": "ada@example.com",
        "email_verified": true,
        "name": "Ada",
        "groups": ["rdc-admins"],
    }))
    .expect("base claims");
    if let serde_json::Value::Object(map) = overrides {
        for (k, v) in map {
            if v.is_null() {
                base.remove(&k);
            } else {
                base.insert(k, v);
            }
        }
    }
    base
}

fn sign(claims: &HashMap<String, serde_json::Value>, key: &EncodingKey) -> String {
    let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(KID.to_string());
    encode(&header, claims, key).expect("sign")
}

fn key_set(jwk: Jwk) -> JwkSet {
    JwkSet { keys: vec![jwk] }
}

// ── the happy path ──────────────────────────────────────────────────────────

#[test]
fn a_well_formed_token_is_accepted() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({})), &key);

    let out = validate_token(&token, &key_set(jwk), &config(), NOW).expect("should validate");

    assert_eq!(out.subject, "user-1");
    assert_eq!(out.email.as_deref(), Some("ada@example.com"));
    assert!(out.email_verified);
    assert_eq!(out.groups, vec!["rdc-admins".to_string()]);
}

// ── rejections ──────────────────────────────────────────────────────────────

#[test]
fn an_expired_token_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "exp": NOW - 3600 })), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("expired"),
        ValidationError::Expired
    );
}

#[test]
fn a_not_yet_valid_token_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "nbf": NOW + 3600 })), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("not yet valid"),
        ValidationError::NotYetValid
    );
}

#[test]
fn a_token_from_another_issuer_is_rejected() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "iss": "https://evil.example.com" })), &key);
    assert!(matches!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("wrong issuer"),
        ValidationError::IssuerMismatch { .. }
    ));
}

#[test]
fn a_token_for_another_audience_is_rejected() {
    // The case that matters most in a shared-IdP estate: a token minted for a
    // different application must not authenticate here.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "aud": "some-other-service" })), &key);
    assert!(matches!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("wrong audience"),
        ValidationError::AudienceMismatch { .. }
    ));
}

#[test]
fn a_token_signed_by_a_different_key_is_rejected() {
    let (_key, jwk) = keypair();
    // Sign with a second, unrelated key while presenting the first key's set.
    let mut rng = <rand_chacha::ChaCha8Rng as rand_core::SeedableRng>::seed_from_u64(99);
    let other = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("second key");
    let der = rsa::pkcs1::EncodeRsaPrivateKey::to_pkcs1_der(&other).expect("encode");
    let other_key = EncodingKey::from_rsa_der(der.as_bytes());
    let token = sign(&claims(json!({})), &other_key);

    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("bad signature"),
        ValidationError::BadSignature
    );
}

#[test]
fn a_token_naming_an_unknown_key_is_rejected() {
    let (key, mut jwk) = keypair();
    let token = sign(&claims(json!({})), &key);
    jwk.kid = "some-other-key".to_string();

    assert!(matches!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("unknown kid"),
        ValidationError::UnknownKid { .. }
    ));
}

#[test]
fn an_unsigned_token_is_rejected() {
    // alg:none is the classic JWT bypass. It must never be verifiable.
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(json!({ "alg": "none", "typ": "JWT", "kid": KID }).to_string());
    let payload = b64.encode(json!(claims(json!({}))).to_string());
    let token = format!("{header}.{payload}.");

    let (_key, jwk) = keypair();
    let err = validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("alg:none");
    assert!(
        matches!(
            err,
            ValidationError::UnsupportedAlgorithm { .. } | ValidationError::Malformed(_)
        ),
        "got {err:?}"
    );
}

#[test]
fn a_symmetrically_signed_token_is_rejected() {
    // HS256 verified against an RSA public key is the other half of the
    // algorithm-confusion attack: the "secret" would be public.
    let mut header = Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some(KID.to_string());
    let token = encode(
        &header,
        &claims(json!({})),
        &EncodingKey::from_secret(b"public-modulus-pretending-to-be-a-secret"),
    )
    .expect("sign");

    let (_key, jwk) = keypair();
    assert!(matches!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("symmetric"),
        ValidationError::UnsupportedAlgorithm { .. }
    ));
}

#[test]
fn a_token_without_a_key_id_is_rejected() {
    let (key, jwk) = keypair();
    let header = Header::new(jsonwebtoken::Algorithm::RS256); // no kid
    let token = encode(&header, &claims(json!({})), &key).expect("sign");

    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("no kid"),
        ValidationError::MissingKid
    );
}

#[test]
fn a_token_without_a_subject_is_rejected() {
    // Identity is keyed on (issuer, subject); without one there is nothing to
    // key on, and falling back to email is exactly what must not happen.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "sub": null })), &key);

    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("no subject"),
        ValidationError::MissingSubject
    );
}

#[test]
fn garbage_is_rejected_without_panicking() {
    let (_key, jwk) = keypair();
    for bad in ["", "not.a.token", "a.b", "....", "eyJhbGciOiJSUzI1NiJ9"] {
        let err = validate_token(bad, &key_set(jwk.clone()), &config(), NOW)
            .expect_err("should not validate");
        assert!(
            matches!(err, ValidationError::Malformed(_)),
            "{bad}: {err:?}"
        );
    }
}

// ── tolerances and claim shapes ─────────────────────────────────────────────

#[test]
fn a_token_just_past_expiry_is_still_accepted_within_clock_skew() {
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "exp": NOW - 30 })), &key);
    validate_token(&token, &key_set(jwk), &config(), NOW)
        .expect("30s of drift must not lock people out");
}

#[test]
fn a_missing_email_verified_claim_counts_as_unverified() {
    // Treating absence as verified would defeat the check email linking rests
    // on, and providers differ about whether they send it.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "email_verified": null })), &key);
    let out = validate_token(&token, &key_set(jwk), &config(), NOW).expect("valid");
    assert!(!out.email_verified);
}

#[test]
fn groups_as_a_bare_string_are_read() {
    // Some providers send one group as a string rather than a single-element
    // array.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "groups": "rdc-admins" })), &key);
    let out = validate_token(&token, &key_set(jwk), &config(), NOW).expect("valid");
    assert_eq!(out.groups, vec!["rdc-admins".to_string()]);
}

#[test]
fn an_unreadable_groups_claim_yields_none_rather_than_a_guess() {
    // With deny-by-default that means rejection, which is the safe direction.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "groups": { "unexpected": true } })), &key);
    let out = validate_token(&token, &key_set(jwk), &config(), NOW).expect("valid");
    assert_eq!(out.groups, Vec::<String>::new());
}

#[test]
fn groups_come_from_the_configured_claim() {
    let (key, jwk) = keypair();
    let token = sign(
        &claims(json!({ "groups": null, "roles": ["from-roles"] })),
        &key,
    );
    let mut config = config();
    config.roles_claim = "roles".to_string();

    let out = validate_token(&token, &key_set(jwk), &config, NOW).expect("valid");
    assert_eq!(out.groups, vec!["from-roles".to_string()]);
}

#[test]
fn a_token_with_no_expiry_is_rejected() {
    // A credential that never expires is not one this system should accept,
    // and absence must not read as permission.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "exp": null })), &key);
    assert_eq!(
        validate_token(&token, &key_set(jwk), &config(), NOW).expect_err("no expiry"),
        ValidationError::MissingExpiry
    );
}

#[test]
fn now_is_authoritative_rather_than_the_system_clock() {
    // The function takes `now` so expiry is testable; if the real clock
    // silently overruled it, every test here would be measuring the wrong
    // thing. A token long expired by wall-clock time must still validate when
    // `now` says otherwise.
    let (key, jwk) = keypair();
    let token = sign(&claims(json!({ "exp": NOW + 60 })), &key);
    validate_token(&token, &key_set(jwk), &config(), NOW)
        .expect("the supplied now must decide, not the system clock");
}
