#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Three servers alive at once: a real `RDataCore`, a mock identity provider,
//! and the MCP HTTP binary pointed at both.
//!
//! All three run in-process. Spawning the MCP binary would make failures
//! opaque — a non-zero exit code and nothing else — and would need a real
//! port rather than an ephemeral one, which is how parallel test runs start
//! colliding.
//!
//! The `RDataCore` under test is configured to trust the mock provider, so a
//! token minted here really does resolve to a real user row, with real roles,
//! through the real provisioning path. That is the point: anything less would
//! be testing the mock.

use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{App, HttpServer};
use base64::Engine as _;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use r_data_core_api::api_state::configure_app;
use r_data_core_mcp::config::Config;
use r_data_core_mcp::http::{self, ServerState};
use r_data_core_test_support::TestDatabase;

use crate::api::workflows::common::build_app_state;

const KID: &str = "mcp-e2e-key";
const AUDIENCE: &str = "rdc-mcp";

/// A running `RDataCore`, identity provider and MCP server.
pub struct OauthStack {
    pub rdc_url: String,
    pub mcp_url: String,
    /// Held, not read: dropping it tears down the test database out from
    /// under the servers still pointing at it.
    _pool: TestDatabase,
    encoding: EncodingKey,
    issuer: String,
    _idp: MockServer,
    _rdc: ServerGuard,
    _mcp: ShutdownGuard,
    /// Declared last so it drops last, after everything that reads the
    /// variables it removes.
    _env: EnvGuard,
}

struct ServerGuard(tokio::task::JoinHandle<()>);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct ShutdownGuard(tokio_util::sync::CancellationToken);

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// Removes the OIDC variables this harness set.
///
/// `build_app_state` reads them from the process environment, so leaving them
/// behind would give every later test in this binary a server configured
/// against a mock provider that has since stopped. Dropped last, after the
/// servers that read them.
struct EnvGuard;

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in ["RDC_OIDC_ISSUER", "RDC_OIDC_AUDIENCE", "RDC_OIDC_ROLE_MAP"] {
            std::env::remove_var(key);
        }
    }
}

impl OauthStack {
    /// Start all three, wired together.
    ///
    /// # Panics
    /// Panics if any of them cannot start. A broken test environment should
    /// fail loudly here rather than as a confusing connection error later.
    pub async fn start(role_map: &str) -> Self {
        let (encoding, jwk) = keypair();
        let idp = MockServer::start().await;
        let issuer = idp.uri();

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issuer": issuer,
                "jwks_uri": format!("{issuer}/jwks"),
                "authorization_endpoint": format!("{issuer}/authorize"),
                "token_endpoint": format!("{issuer}/token"),
            })))
            .mount(&idp)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [jwk] })))
            .mount(&idp)
            .await;

        // RDataCore reads its OIDC settings from the process environment at
        // startup, so they must be set before the state is built. Tests using
        // this harness are serialised for that reason.
        std::env::set_var("RDC_OIDC_ISSUER", &issuer);
        std::env::set_var("RDC_OIDC_AUDIENCE", AUDIENCE);
        std::env::set_var("RDC_OIDC_ROLE_MAP", role_map);

        let (app_data, pool, _token, _api_key) =
            build_app_state().await.expect("build application state");

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("an ephemeral port");
        let rdc_port = listener.local_addr().expect("the bound port").port();
        let server = HttpServer::new(move || {
            App::new()
                .app_data(app_data.clone())
                .configure(configure_app)
        })
        .listen(listener)
        .expect("attach the server")
        .workers(1)
        .run();
        let rdc = ServerGuard(tokio::spawn(async move {
            let _ = server.await;
        }));
        let rdc_url = format!("http://127.0.0.1:{rdc_port}");

        // The MCP server, pointed at both.
        let mcp_listener = http::bind("127.0.0.1:0").await.expect("an ephemeral port");
        let mcp_addr = mcp_listener.local_addr().expect("the bound address");
        let mcp_url = format!("http://{mcp_addr}");

        let mut pairs = HashMap::new();
        pairs.insert("RDC_BASE_URL".to_string(), rdc_url.clone());
        pairs.insert("RDC_MCP_TRANSPORT".to_string(), "http".to_string());
        pairs.insert("RDC_MCP_RESOURCE_URL".to_string(), mcp_url.clone());
        pairs.insert("RDC_OIDC_ISSUER".to_string(), issuer.clone());
        pairs.insert("RDC_OIDC_AUDIENCE".to_string(), AUDIENCE.to_string());
        let config = Config::from_map(&pairs).expect("a valid MCP configuration");

        let state = Arc::new(ServerState::new(config).expect("MCP server state"));
        let shutdown = tokio_util::sync::CancellationToken::new();
        let serving = shutdown.clone();
        tokio::spawn(async move {
            let _ = http::serve(state, mcp_listener, serving).await;
        });

        Self {
            rdc_url,
            mcp_url,
            _pool: pool,
            encoding,
            issuer,
            _idp: idp,
            _rdc: rdc,
            _mcp: ShutdownGuard(shutdown),
            _env: EnvGuard,
        }
    }

    /// A signed identity token for a named person in named groups.
    pub fn mint_token(&self, subject: &str, groups: &[&str]) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let claims = json!({
            "iss": self.issuer,
            "aud": AUDIENCE,
            "sub": subject,
            "exp": now + 3600,
            "iat": now,
            "email": format!("{subject}@example.com"),
            "email_verified": true,
            "name": subject,
            "groups": groups,
        });

        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(KID.to_string());
        encode(&header, &claims, &self.encoding).expect("sign a token")
    }
}

/// A 2048-bit RSA key, fixed across runs so a failure reproduces.
fn keypair() -> (EncodingKey, serde_json::Value) {
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    let mut rng = <rand_chacha::ChaCha8Rng as rand_core::SeedableRng>::seed_from_u64(7);
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
