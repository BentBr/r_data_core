#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Serving MCP over HTTP as an OAuth 2.1 resource server.
//!
//! Three things share one listener, and the difference between them matters:
//!
//! - `/.well-known/oauth-protected-resource` is **unauthenticated**. A client
//!   cannot authenticate until it has read this, so requiring a token here
//!   would deadlock discovery.
//! - `/health` is unauthenticated, for load balancers.
//! - `/mcp` requires a valid bearer, and answers `401` with the discovery
//!   challenge when it does not get one.
//!
//! **Sessions are off.** `rmcp` can keep a session alive across requests,
//! which would mean the tool instance built for the caller who opened the
//! session serves every later request on it. Anyone who obtained that session
//! id would then act as the caller who created it. Running statelessly costs
//! a permission lookup per request — cached — and removes the question
//! entirely.
//!
//! **`Origin` is checked before anything else.** `rmcp` can do this too, but
//! only for requests that reach its handler — which means after this module's
//! authentication, and never for the metadata and health endpoints, which do
//! not go through it at all. Checking here covers every path and makes the
//! rejection observable, which is what lets a test prove the protection is on
//! rather than merely configured.
//!
//! **Permissions are per caller, never per process.** They are cached under a
//! digest of the caller's own token; a process-wide cache would let one
//! user's tool visibility leak to the next connection.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::sync::RwLock;

use r_data_core_core::oidc::keys::KeySource;
use r_data_core_core::oidc::{validate_token, OidcConfig};
use r_data_core_services::oidc_keys::HttpKeySource;

use crate::auth::{metadata, CallerContext, OidcBackend, Permissions};
use crate::client::RdcClient;
use crate::config::Config;
use crate::tools::RdcTools;

/// Where the MCP endpoint lives.
pub const MCP_PATH: &str = "/mcp";
/// Liveness probe path.
pub const HEALTH_PATH: &str = "/health";

/// How long a caller's permissions are reused.
///
/// Short, because it is how long a revoked role keeps a tool visible. Tool
/// visibility is only ergonomics — `RDataCore` re-checks every call — so this
/// is a latency trade, not a security one.
const PERMISSIONS_TTL: Duration = Duration::from_secs(60);

/// The body type `rmcp`'s tower service produces.
type ResponseBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// Permissions held for one caller.
struct CachedPermissions {
    permissions: Permissions,
    fetched_at: std::time::Instant,
}

/// Everything a request needs, built once at startup.
pub struct ServerState {
    config: Config,
    oidc: OidcConfig,
    keys: Arc<dyn KeySource>,
    client: Arc<RdcClient>,
    permissions: RwLock<HashMap<String, CachedPermissions>>,
    http_config: StreamableHttpServerConfig,
}

impl ServerState {
    /// # Errors
    /// Returns an error when the configuration is not a usable HTTP
    /// configuration, or a client cannot be built.
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let oidc = config
            .oidc_config()
            .ok_or("http transport requires RDC_OIDC_ISSUER and RDC_OIDC_AUDIENCE")?;

        let keys: Arc<dyn KeySource> = Arc::new(HttpKeySource::new(&oidc)?);
        let backend = Arc::new(OidcBackend::new(&config)?);
        let client = Arc::new(RdcClient::new(&config, backend)?);

        // Built by mutation rather than a struct literal: the type is
        // `#[non_exhaustive]`, so a literal will not compile.
        let mut http_config = StreamableHttpServerConfig::default();
        // See the module comment: a session outliving one caller's token is an
        // impersonation surface, and stateless costs little.
        http_config.legacy_session_mode = false;
        http_config.json_response = true;
        // The specification requires Origin validation against DNS rebinding,
        // and rmcp's default list is empty — which disables the check. An
        // unset allowlist is therefore a hole, not a permissive default.
        http_config
            .allowed_origins
            .clone_from(&config.allowed_origins);
        if !config.allowed_hosts.is_empty() {
            http_config.allowed_hosts.clone_from(&config.allowed_hosts);
        }

        Ok(Self {
            config,
            oidc,
            keys,
            client,
            permissions: RwLock::new(HashMap::new()),
            http_config,
        })
    }

    /// Validate an inbound bearer and say who it belongs to.
    ///
    /// Validation happens here, exactly once per request. The resulting
    /// context is marked validated, and the auth backend refuses to act on one
    /// that is not.
    async fn authenticate(&self, token: &str) -> Result<CallerContext, String> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let keys = self
            .keys
            .keys()
            .await
            .map_err(|e| format!("could not obtain the provider's signing keys: {e}"))?;

        // A rotated key is normal, so an unknown key id earns one refresh.
        let verified = match validate_token(token, &keys, &self.oidc, now) {
            Err(r_data_core_core::oidc::ValidationError::UnknownKid { kid }) => {
                let refreshed = self
                    .keys
                    .refresh_for_unknown_kid(&kid)
                    .await
                    .map_err(|e| format!("could not refresh the signing keys: {e}"))?;
                validate_token(token, &refreshed, &self.oidc, now)
            }
            other => other,
        }
        .map_err(|e| e.to_string())?;

        log::debug!(
            "authenticated an MCP caller from {} (subject length {})",
            verified.issuer,
            verified.subject.len()
        );

        Ok(CallerContext {
            bearer: Some(token.to_string()),
            validated: true,
        })
    }

    /// What this caller may do, from cache when it is fresh.
    async fn permissions_for(&self, ctx: &CallerContext) -> Permissions {
        let key = ctx.bearer.as_deref().map_or_else(String::new, digest);

        {
            let cache = self.permissions.read().await;
            if let Some(hit) = cache
                .get(&key)
                .filter(|entry| entry.fetched_at.elapsed() < PERMISSIONS_TTL)
            {
                return hit.permissions.clone();
            }
        }

        // An empty permission set on failure rather than a full one: being
        // offered nothing is a visible, self-explanatory problem, while being
        // offered everything produces a stream of 403s the model cannot act on.
        let permissions = match self.client.permissions(ctx).await {
            Ok(body) => Permissions::from_api_response(&body),
            Err(e) => {
                log::warn!(
                    "could not load permissions for an MCP caller: {}",
                    e.to_tool_message()
                );
                Permissions::default()
            }
        };

        self.permissions.write().await.insert(
            key,
            CachedPermissions {
                permissions: permissions.clone(),
                fetched_at: std::time::Instant::now(),
            },
        );
        permissions
    }
}

/// Serve until `shutdown` fires.
///
/// # Errors
/// Returns an error if the listener cannot be bound.
pub async fn serve(
    state: Arc<ServerState>,
    listener: tokio::net::TcpListener,
    shutdown: tokio_util::sync::CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let accepted = tokio::select! {
            () = shutdown.cancelled() => return Ok(()),
            accepted = listener.accept() => accepted,
        };

        let (stream, peer) = match accepted {
            Ok(pair) => pair,
            Err(e) => {
                log::warn!("could not accept a connection: {e}");
                continue;
            }
        };

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { Ok::<_, Infallible>(route(state, req).await) }
            });

            if let Err(e) =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection_with_upgrades(hyper_util::rt::TokioIo::new(stream), service)
                    .await
            {
                log::debug!("connection from {peer} ended: {e}");
            }
        });
    }
}

/// Bind the configured address.
///
/// # Errors
/// Returns an error if the address is unusable.
pub async fn bind(addr: &str) -> Result<tokio::net::TcpListener, std::io::Error> {
    tokio::net::TcpListener::bind(addr).await
}

/// Route one request.
async fn route(state: Arc<ServerState>, req: Request<Incoming>) -> Response<ResponseBody> {
    let path = req.uri().path().to_string();

    // Before anything else, including the unauthenticated endpoints: a
    // DNS-rebinding attack aims a browser at a server it should not be able to
    // reach, and discovery metadata is exactly what it would want to read.
    if !origin_allowed(&state.config.allowed_origins, &req) {
        return text_response(StatusCode::FORBIDDEN, "origin not allowed");
    }

    if path == metadata::METADATA_PATH {
        return json_response(
            StatusCode::OK,
            &metadata::protected_resource_document(&state.config),
        );
    }
    if path == HEALTH_PATH {
        return text_response(StatusCode::OK, "ok");
    }
    if path != MCP_PATH {
        return text_response(StatusCode::NOT_FOUND, "not found");
    }

    let Some(token) = bearer_of(&req) else {
        return challenge(&state);
    };

    let ctx = match state.authenticate(&token).await {
        Ok(ctx) => ctx,
        Err(reason) => {
            // Logged in full, answered with almost nothing: which part of a
            // forgery was wrong is not something to hand back.
            log::warn!("rejected an MCP caller: {reason}");
            return challenge(&state);
        }
    };

    let permissions = state.permissions_for(&ctx).await;
    let client = Arc::clone(&state.client);

    // A fresh tool instance per request, holding this caller's context and
    // permissions. This is what Task 11a's `caller()` accessor made possible:
    // no tool body knows the transport changed.
    let service = StreamableHttpService::new(
        move || {
            Ok(RdcTools::new(
                Arc::clone(&client),
                permissions.clone(),
                ctx.clone(),
            ))
        },
        Arc::new(NeverSessionManager::default()),
        state.http_config.clone(),
    );

    service.handle(req).await
}

/// A `401` carrying the discovery challenge.
///
/// The challenge is the whole point: without it a client knows only that it
/// needs a token, not where to obtain one.
fn challenge(state: &ServerState) -> Response<ResponseBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            "WWW-Authenticate",
            metadata::challenge_header(&state.config),
        )
        .header("Content-Type", "text/plain")
        .body(boxed("authentication required"))
        .unwrap_or_else(|_| Response::new(boxed("authentication required")))
}

/// Whether a request's `Origin` is one this server serves.
///
/// An empty allowlist means the check is off, matching `rmcp`'s own
/// behaviour — a non-browser deployment has no `Origin` to check and should
/// not have to invent one. A request carrying no `Origin` passes: that is a
/// non-browser client, which the header exists to distinguish from.
///
/// Comparison is exact on the whole `scheme://host[:port]` string. Matching a
/// suffix would let `https://evil-example.com` past an allowlist naming
/// `https://example.com`, which is the classic way this check is got wrong.
fn origin_allowed(allowed: &[String], req: &Request<Incoming>) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let Some(origin) = req
        .headers()
        .get(hyper::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        return true;
    };
    allowed.iter().any(|permitted| permitted == origin)
}

/// The bearer token on a request, if there is a well-formed one.
fn bearer_of(req: &Request<Incoming>) -> Option<String> {
    req.headers()
        .get(hyper::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn json_response(status: StatusCode, body: &serde_json::Value) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(boxed(&body.to_string()))
        .unwrap_or_else(|_| Response::new(boxed("{}")))
}

fn text_response(status: StatusCode, body: &str) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(boxed(body))
        .unwrap_or_else(|_| Response::new(boxed(body)))
}

fn boxed(body: &str) -> ResponseBody {
    Full::new(Bytes::from(body.to_string()))
        .map_err(|never| match never {})
        .boxed()
}

/// A digest of a caller's token, for use as a cache key.
///
/// Never the token itself: a cache key ends up in logs and metrics far more
/// readily than a value does.
fn digest(token: &str) -> String {
    use sha2::{Digest as _, Sha256};
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// The address the server should listen on.
#[must_use]
pub fn bind_address(config: &Config) -> String {
    match &config.transport {
        crate::config::Transport::Http { bind } => bind.clone(),
        crate::config::Transport::Stdio => String::new(),
    }
}

/// Log what is being served, once, at startup.
pub fn log_startup(config: &Config, addr: SocketAddr) {
    log::info!(
        "MCP server listening on {addr} against {} — metadata at {}, MCP at {MCP_PATH}",
        config.base_url,
        metadata::METADATA_PATH
    );
}

#[cfg(test)]
mod http_tests;
