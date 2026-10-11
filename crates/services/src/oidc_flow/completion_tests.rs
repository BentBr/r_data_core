#![allow(clippy::expect_used, clippy::unwrap_used)]

//! A sign-in that goes all the way through: a signed ID token from the mock
//! provider, verified against its real key, resolved to roles.
//!
//! `oidc_flow/tests` stops at the provider's door. These tests are the
//! other half, and they exist because the bug they guard against — a
//! completed sign-in that forgets which roles it mapped to — is invisible from
//! anywhere short of the end.

use std::collections::HashMap;
use std::sync::Arc;

use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::oidc::keys::Jwk;
use r_data_core_core::oidc::{JwkSet, OidcConfig, StaticKeySource};

use super::*;
use crate::oidc_test_fakes::{service, FakeIdentities, FakeUsers};

const KID: &str = "completion-key";
const AUDIENCE: &str = "r-data-core";

/// A fixed RSA key, so a failure reproduces.
fn keypair() -> (EncodingKey, Jwk) {
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    let mut rng = <rand_chacha::ChaCha8Rng as rand_core::SeedableRng>::seed_from_u64(11);
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

/// A provider whose token endpoint answers with an ID token built by
/// `id_token`, which the test configures per call.
async fn provider() -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": uri,
            "jwks_uri": format!("{uri}/jwks"),
            "authorization_endpoint": format!("{uri}/authorize"),
            "token_endpoint": format!("{uri}/token"),
        })))
        .mount(&server)
        .await;
    server
}

/// Mount a token endpoint that answers with this ID token.
async fn token_endpoint(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": id_token })))
        .mount(server)
        .await;
}

/// A signed ID token for `subject` in `groups`, with `extra` claims merged in.
fn id_token(issuer: &str, key: &EncodingKey, subject: &str, extra: &serde_json::Value) -> String {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut claims: HashMap<String, serde_json::Value> = serde_json::from_value(json!({
        "iss": issuer,
        "aud": AUDIENCE,
        "sub": subject,
        "exp": now + 3600,
        "iat": now,
        "email": format!("{subject}@example.com"),
        "email_verified": true,
        "groups": ["rdc-ops"],
    }))
    .expect("base claims");
    if let serde_json::Value::Object(fields) = extra {
        for (k, v) in fields {
            if v.is_null() {
                claims.remove(k);
            } else {
                claims.insert(k.clone(), v.clone());
            }
        }
    }
    let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(KID.to_string());
    encode(&header, &claims, key).expect("sign")
}

fn flow(server: &MockServer, jwk: Jwk) -> OidcFlow {
    let pairs: HashMap<String, String> = [
        ("RDC_OIDC_ISSUER", server.uri()),
        ("RDC_OIDC_AUDIENCE", AUDIENCE.to_string()),
        ("RDC_OIDC_ROLE_MAP", "rdc-ops:editor".to_string()),
        ("RDC_OIDC_CLIENT_ID", "rdc".to_string()),
        (
            "RDC_OIDC_REDIRECT_URI",
            "https://rdc.example.com/admin/api/v1/auth/oidc/callback".to_string(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let config = OidcConfig::from_map(&pairs)
        .expect("valid")
        .expect("enabled");
    let cache = Arc::new(CacheManager::new(CacheConfig::default()));
    let runtime = Arc::new(OidcRuntime::new(
        config,
        Arc::new(StaticKeySource::new(JwkSet { keys: vec![jwk] })),
        Arc::new(service(
            Arc::new(FakeIdentities::default()),
            Arc::new(FakeUsers::default()),
        )),
        Arc::clone(&cache),
    ));
    OidcFlow::new(runtime, cache, None).expect("flow")
}

/// The `nonce` the flow sent to the provider.
fn nonce_of(url: &str) -> String {
    url.split('?')
        .nth(1)
        .expect("a query")
        .split('&')
        .find_map(|pair| pair.strip_prefix("nonce="))
        .expect("a nonce")
        .to_string()
}

#[tokio::test]
async fn a_completed_sign_in_carries_the_roles_its_groups_map_to() {
    let (key, jwk) = keypair();
    let server = provider().await;
    let flow = flow(&server, jwk);

    let redirect = flow.start(None).await.expect("start");
    let nonce = nonce_of(&redirect.url);
    token_endpoint(
        &server,
        &id_token(&server.uri(), &key, "ada", &json!({ "nonce": nonce })),
    )
    .await;

    let completed = flow
        .complete(&redirect.state, "code", 1800)
        .await
        .expect("a genuine sign-in must complete");

    assert_eq!(completed.identity.subject, "ada");
    assert_eq!(completed.claims.name, "ada");
    assert_eq!(
        completed
            .roles
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        vec!["editor"],
        "the session must be minted from the mapped roles, not from user_roles"
    );
}

#[tokio::test]
async fn start_sends_a_nonce_and_stores_it_for_the_callback() {
    let (_, jwk) = keypair();
    let server = provider().await;
    let redirect = flow(&server, jwk).start(None).await.expect("start");
    assert!(
        redirect.url.contains("&nonce="),
        "the nonce must travel to the provider: {}",
        redirect.url
    );
}

#[tokio::test]
async fn an_id_token_with_the_wrong_nonce_is_refused() {
    let (key, jwk) = keypair();
    let server = provider().await;
    let flow = flow(&server, jwk);

    let redirect = flow.start(None).await.expect("start");
    token_endpoint(
        &server,
        &id_token(
            &server.uri(),
            &key,
            "ada",
            &json!({ "nonce": "not-the-one-we-sent" }),
        ),
    )
    .await;

    let outcome = flow.complete(&redirect.state, "code", 1800).await;
    assert!(
        matches!(
            outcome,
            Err(FlowError::Auth(OidcAuthError::Rejected(
                r_data_core_core::oidc::ValidationError::NonceMismatch
            )))
        ),
        "expected a nonce refusal, got {outcome:?}"
    );
}

#[tokio::test]
async fn an_id_token_without_a_nonce_is_refused() {
    let (key, jwk) = keypair();
    let server = provider().await;
    let flow = flow(&server, jwk);

    let redirect = flow.start(None).await.expect("start");
    token_endpoint(&server, &id_token(&server.uri(), &key, "ada", &json!({}))).await;

    assert!(
        flow.complete(&redirect.state, "code", 1800).await.is_err(),
        "a token obtained outside this attempt carries no nonce and must be refused"
    );
}
