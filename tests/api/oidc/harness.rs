#![allow(clippy::expect_used, clippy::unwrap_used)]
// Test scaffolding: panicking is how it reports a broken fixture, and nothing
// here is called for a value it would be a mistake to discard.
#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! A mock identity provider and an app wired to trust it.
//!
//! Deliberately built without a database, a queue or Redis. Every refusal the
//! middleware makes — an expired token, a wrong audience, a forged signature,
//! a token for another service — happens before anything is looked up, so
//! these tests prove that ordering rather than assuming it. If a rejection
//! path ever started reaching for the database, it would fail here instead of
//! quietly working in an environment that happens to have one.
//!
//! The state is a double implementing only what this path touches: the OIDC
//! runtime and the local JWT secret. Everything else is deliberately absent,
//! which is what keeps the tests honest about how narrow the middleware's
//! dependencies are.

use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{web, HttpResponse};
use base64::Engine as _;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use r_data_core_api::api_state::{ApiStateTrait, ApiStateWrapper};
use r_data_core_api::auth::auth_enum::RequiredAuth;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::oidc::OidcConfig;
use r_data_core_persistence::{AdminUserRepository, IdentityRepository, RoleRepository};
use r_data_core_services::oidc_keys::HttpKeySource;
use r_data_core_services::oidc_provisioning::OidcProvisioningService;
use r_data_core_services::OidcRuntime;

pub const AUDIENCE: &str = "r-data-core";
pub const KID: &str = "test-key-1";
pub const JWT_SECRET: &str = "test_secret";

// ── the mock provider ───────────────────────────────────────────────────────

/// A mock provider, and the key it signs with.
pub struct Idp {
    server: MockServer,
    encoding: EncodingKey,
}

impl Idp {
    /// Stand up a provider serving discovery and a key set.
    pub async fn start() -> Self {
        let (encoding, jwk) = keypair();
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "jwks_uri": format!("{}/jwks", server.uri()) })),
            )
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [jwk] })))
            .mount(&server)
            .await;

        Self { server, encoding }
    }

    pub fn issuer(&self) -> String {
        self.server.uri()
    }

    /// Sign a token, starting from a valid one and applying overrides.
    ///
    /// Overrides rather than whole payloads, so each test states only the one
    /// thing it is making wrong and a reader can see it at a glance. A `null`
    /// override removes the claim.
    pub fn token(&self, overrides: &serde_json::Value) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let mut claims: HashMap<String, serde_json::Value> = serde_json::from_value(json!({
            "iss": self.issuer(),
            "aud": AUDIENCE,
            "sub": "ada",
            "exp": now + 3600,
            "iat": now,
            "email": "ada@example.com",
            "email_verified": true,
            "name": "Ada Lovelace",
            "groups": ["rdc-ops"],
        }))
        .expect("base claims");

        if let serde_json::Value::Object(fields) = overrides {
            for (key, value) in fields {
                if value.is_null() {
                    claims.remove(key);
                } else {
                    claims.insert(key.clone(), value.clone());
                }
            }
        }

        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(KID.to_string());
        encode(&header, &claims, &self.encoding).expect("sign")
    }
}

/// A 2048-bit RSA key, fixed across runs so a failure reproduces.
fn keypair() -> (EncodingKey, serde_json::Value) {
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    let mut rng = <rand_chacha::ChaCha8Rng as rand_core::SeedableRng>::seed_from_u64(42);
    let private = RsaPrivateKey::new(&mut rng, 2048).expect("generate an RSA key");
    let der = private.to_pkcs1_der().expect("encode the private key");
    let encoding = EncodingKey::from_rsa_der(der.as_bytes());

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let jwk = json!({
        "kid": KID,
        "kty": "RSA",
        "alg": "RS256",
        "n": b64.encode(private.n().to_bytes_be()),
        "e": b64.encode(private.e().to_bytes_be()),
    });
    (encoding, jwk)
}

// ── the state double ────────────────────────────────────────────────────────

/// The narrowest state the OIDC arm can run against.
///
/// Only `jwt_secret` and `oidc_runtime` are real. The rest return a unit
/// value, so any route that reaches for a service gets `None` from the
/// downcast rather than a plausible-looking fake — a test that wanders
/// outside this path should fail loudly.
struct MinimalState {
    pool: PgPool,
    nothing: (),
    oidc_runtime: Option<Arc<OidcRuntime>>,
}

impl ApiStateTrait for MinimalState {
    fn db_pool(&self) -> &PgPool {
        &self.pool
    }
    fn jwt_secret(&self) -> &str {
        JWT_SECRET
    }
    fn api_key_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn admin_user_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn role_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn api_config_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn entity_definition_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn dynamic_entity_service_ref(&self) -> Option<&dyn std::any::Any> {
        None
    }
    fn cache_manager_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn workflow_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn queue_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn dashboard_stats_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn license_service_ref(&self) -> &dyn std::any::Any {
        &self.nothing
    }
    fn password_reset_service_ref(&self) -> Option<&dyn std::any::Any> {
        None
    }
    fn system_log_service_ref(&self) -> Option<&dyn std::any::Any> {
        None
    }
    fn oidc_runtime_ref(&self) -> Option<&dyn std::any::Any> {
        self.oidc_runtime
            .as_ref()
            .map(|rt| rt as &dyn std::any::Any)
    }
}

/// A pool that never connects unless a query is actually run.
///
/// Reaching the database is therefore a connection error, which is how the
/// "the provider is fine, we are not" case gets asserted.
fn lazy_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(250))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .expect("a lazy pool needs no server")
}

/// Build the state, trusting `idp` when one is given.
pub fn state(idp: Option<&Idp>) -> ApiStateWrapper {
    let pool = lazy_pool();

    let oidc_runtime = idp.map(|idp| {
        let config = OidcConfig::from_map(
            &[
                ("RDC_OIDC_ISSUER".to_string(), idp.issuer()),
                ("RDC_OIDC_AUDIENCE".to_string(), AUDIENCE.to_string()),
                (
                    "RDC_OIDC_ROLE_MAP".to_string(),
                    "rdc-ops:editor".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .expect("valid")
        .expect("enabled");

        let keys = Arc::new(HttpKeySource::new(&config).expect("key source"));
        let provisioning = Arc::new(OidcProvisioningService::new(
            Arc::new(IdentityRepository::new(pool.clone())),
            Arc::new(AdminUserRepository::new(Arc::new(pool.clone()))),
            Arc::new(RoleRepository::new(pool.clone())),
        ));
        Arc::new(OidcRuntime::new(
            config,
            keys,
            provisioning,
            Arc::new(CacheManager::new(CacheConfig::default())),
        ))
    });

    ApiStateWrapper::new(MinimalState {
        pool,
        nothing: (),
        oidc_runtime,
    })
}

// ── the protected route ─────────────────────────────────────────────────────

/// The one route these tests need: it answers with whoever the request turned
/// out to be, so authentication can be told apart from authorization.
pub async fn whoami(auth: RequiredAuth) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "name": auth.0.name,
        "permissions": auth.0.permissions,
    }))
}

/// Register the protected route behind the OIDC arm.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/whoami")
            .wrap(r_data_core_api::middleware::OidcAuth::new())
            .to(whoami),
    );
}
