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
//! **`Host` and `Origin` are both checked before anything else.** `rmcp` can
//! check them too, but only for requests that reach its handler — which means
//! after this module's authentication, and never for the metadata and health
//! endpoints, which do not go through it at all. A DNS-rebinding attack aims a
//! browser at a server it should not reach, and discovery metadata is exactly
//! what it would want to read, so the check has to sit in front of everything.
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
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
use tokio::sync::RwLock;

use r_data_core_core::oidc::keys::KeySource;
use r_data_core_core::oidc::{validate_token, OidcConfig};
use r_data_core_oidc_http::HttpKeySource;

use crate::auth::{metadata, CallerContext, OidcBackend, Permissions};
use crate::client::RdcClient;
use crate::config::Config;

mod guards;
mod routes;
pub mod server;

pub use server::serve;

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

        // A client that follows the specification reads the advertised
        // `resource` from the metadata document and asks its provider for a
        // token scoped to it. If that is not the audience this server checks,
        // the client does everything right and is rejected anyway — with a 401
        // that says nothing about why. Cheap to detect, miserable to diagnose.
        if let (Some(resource), Some(audience)) = (&config.resource_url, &config.oidc_audience) {
            if resource != audience {
                log::warn!(
                    "RDC_MCP_RESOURCE_URL ({resource}) and RDC_OIDC_AUDIENCE ({audience}) \
                     differ. The metadata document advertises the former, but tokens are \
                     validated against the latter — a client that requests a token for the \
                     advertised resource will be refused. Set them to the same value unless \
                     your provider cannot mint a URL audience."
                );
            }
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
    pub(super) async fn authenticate(&self, token: &str) -> Result<CallerContext, String> {
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
    pub(super) async fn permissions_for(&self, ctx: &CallerContext) -> Permissions {
        let key = ctx
            .bearer
            .as_deref()
            .map_or_else(String::new, guards::digest);

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

        let mut cache = self.permissions.write().await;
        // Evict what has aged out first: nothing else removes an entry, and
        // one per distinct token seen since startup would grow without bound
        // on a long-running server.
        cache.retain(|_, held| held.fetched_at.elapsed() < PERMISSIONS_TTL);
        cache.insert(
            key,
            CachedPermissions {
                permissions: permissions.clone(),
                fetched_at: std::time::Instant::now(),
            },
        );
        drop(cache);

        permissions
    }
}

impl ServerState {
    /// The configuration this server was built from.
    pub(crate) const fn config(&self) -> &Config {
        &self.config
    }

    /// The accept-loop limits.
    pub(crate) const fn limits(&self) -> &server::HttpServerLimits {
        &self.config.server_limits
    }

    /// The `rmcp` transport settings.
    pub(crate) const fn http_config(&self) -> &StreamableHttpServerConfig {
        &self.http_config
    }

    /// The client every request's tools share.
    pub(crate) const fn client(&self) -> &Arc<RdcClient> {
        &self.client
    }
}

/// Bind the configured address.
///
/// # Errors
/// Returns an error if the address is unusable.
pub async fn bind(addr: &str) -> Result<tokio::net::TcpListener, std::io::Error> {
    tokio::net::TcpListener::bind(addr).await
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
