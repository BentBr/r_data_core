#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Configuration, and the combinations it refuses.
//!
//! Most of this file is ordinary environment parsing. The part that matters is
//! the transport/credential match, which is exhaustive on purpose: an earlier
//! draft used a catch-all arm, and the combination it silently allowed —
//! HTTP transport with neither a credential nor an issuer — would have started
//! an unauthenticated server on a port.

use std::collections::HashMap;
use std::time::Duration;

use thiserror::Error;
use url::Url;

/// Outbound HTTP timeout when unset.
const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Listen address when unset. Loopback, so an unconfigured server is not
/// reachable from the network.
const DEFAULT_BIND: &str = "127.0.0.1:8931";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("RDC_BASE_URL is required (the base URL of your `RDataCore` instance)")]
    MissingBaseUrl,
    #[error("RDC_BASE_URL is not a valid URL: {0}")]
    InvalidBaseUrl(String),
    #[error("RDC_MCP_TRANSPORT must be 'stdio' or 'http', got '{0}'")]
    InvalidTransport(String),
    #[error("stdio transport requires RDC_API_KEY")]
    MissingCredential,
    #[error(
        "http transport requires RDC_OIDC_ISSUER: without it every caller would be \
         unauthenticated. Use stdio transport for single-user local access."
    )]
    HttpRequiresOidc,
    #[error(
        "http transport with a static RDC_API_KEY would authenticate every caller as the \
         same account, collapsing per-user permissions. Remove RDC_API_KEY and rely on \
         RDC_OIDC_ISSUER, or use stdio transport."
    )]
    SharedCredentialOverHttp,
    #[error(
        "RDC_OIDC_ISSUER requires RDC_OIDC_AUDIENCE: a token minted for another service \
         that happens to share the issuer must not be accepted"
    )]
    MissingAudience,
    #[error("http transport requires RDC_MCP_RESOURCE_URL (this server's own public URL, for OAuth discovery)")]
    MissingResourceUrl,
    #[error("RDC_MCP_RESOURCE_URL is not a valid URL: {0}")]
    InvalidResourceUrl(String),
    #[error("RDC_MCP_RESOURCE_URL must use https (or be on localhost for development): bearer tokens must not cross plaintext")]
    InsecureResourceUrl,
    #[error("{0} must be a positive integer, got '{1}'")]
    InvalidNumber(&'static str, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    Http { bind: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: Url,
    pub transport: Transport,
    /// Static credential for stdio mode. Never set in OIDC mode.
    pub credential: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_audience: Option<String>,
    /// This server's own public URL, for OAuth protected-resource metadata.
    pub resource_url: Option<String>,
    /// Browser origins permitted to reach the MCP endpoint.
    ///
    /// The specification requires `Origin` validation against DNS-rebinding
    /// attacks, and `rmcp` disables the check when the list is empty — so an
    /// unset allowlist is a hole rather than a permissive default. Defaults to
    /// the origin of `resource_url`, which is the only origin a correctly
    /// configured deployment serves from.
    pub allowed_origins: Vec<String>,
    /// `Host` authorities permitted to reach the MCP endpoint.
    pub allowed_hosts: Vec<String>,
    pub timeout: Duration,
}

impl Config {
    /// Read configuration from the process environment.
    ///
    /// # Errors
    /// Returns `ConfigError` if a required variable is missing or malformed, or
    /// if the combination of transport and credentials would be unsafe.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_map(&std::env::vars().collect())
    }

    /// Read configuration from an explicit map.
    ///
    /// Exists so the refusal rules are testable without mutating process-global
    /// state, which no test can do safely in parallel.
    ///
    /// # Errors
    /// As [`Self::from_env`].
    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let base_url = Url::parse(get(map, "RDC_BASE_URL").ok_or(ConfigError::MissingBaseUrl)?)
            .map_err(|e| ConfigError::InvalidBaseUrl(e.to_string()))?;

        let transport = match get(map, "RDC_MCP_TRANSPORT") {
            None | Some("stdio") => Transport::Stdio,
            Some("http") => Transport::Http {
                bind: get(map, "RDC_MCP_BIND").unwrap_or(DEFAULT_BIND).to_string(),
            },
            Some(other) => return Err(ConfigError::InvalidTransport(other.to_string())),
        };

        let credential = get(map, "RDC_API_KEY").map(str::to_string);
        let oidc_issuer = get(map, "RDC_OIDC_ISSUER").map(str::to_string);
        let oidc_audience = get(map, "RDC_OIDC_AUDIENCE").map(str::to_string);

        // Exhaustive on purpose — see the module comment.
        match (&transport, credential.is_some(), oidc_issuer.is_some()) {
            (Transport::Stdio, false, _) => return Err(ConfigError::MissingCredential),
            (Transport::Http { .. }, _, false) => return Err(ConfigError::HttpRequiresOidc),
            (Transport::Http { .. }, true, true) => {
                return Err(ConfigError::SharedCredentialOverHttp)
            }
            // The two safe shapes: stdio with a credential, http with an
            // issuer and no shared credential. Listed together rather than
            // separately only to satisfy `match_same_arms`; the match is still
            // exhaustive, so a new transport cannot slip through unexamined.
            (Transport::Stdio, true, _) | (Transport::Http { .. }, false, true) => {}
        }

        if oidc_issuer.is_some() && oidc_audience.is_none() {
            return Err(ConfigError::MissingAudience);
        }

        let resource_url = get(map, "RDC_MCP_RESOURCE_URL").map(str::to_string);
        if matches!(transport, Transport::Http { .. }) {
            validate_resource_url(resource_url.as_deref())?;
        }

        let timeout = match get(map, "RDC_MCP_TIMEOUT_SECS") {
            None => Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            Some(raw) => raw.parse::<u64>().map_or_else(
                |_| {
                    Err(ConfigError::InvalidNumber(
                        "RDC_MCP_TIMEOUT_SECS",
                        raw.to_string(),
                    ))
                },
                |secs| Ok(Duration::from_secs(secs)),
            )?,
        };

        let allowed_origins = get(map, "RDC_MCP_ALLOWED_ORIGINS").map_or_else(
            || origin_of(resource_url.as_deref()).into_iter().collect(),
            split_list,
        );
        // Derived: the public authority plus loopback. Both are needed. The
        // public one is how a deployed server is reached; loopback is how it
        // is reached in development and behind a proxy that does not preserve
        // the original Host — and rmcp's own default is loopback-only for the
        // same reason. Neither weakens the DNS-rebinding protection this
        // exists for: an attacker's name resolving to 127.0.0.1 still arrives
        // with its own Host and is refused.
        let allowed_hosts = get(map, "RDC_MCP_ALLOWED_HOSTS").map_or_else(
            || {
                let mut hosts: Vec<String> = ["localhost", "127.0.0.1", "[::1]"]
                    .iter()
                    .map(|h| (*h).to_string())
                    .collect();
                if let Some(authority) = authority_of(resource_url.as_deref()) {
                    hosts.push(authority);
                }
                hosts
            },
            split_list,
        );

        Ok(Self {
            base_url,
            transport,
            credential,
            oidc_issuer,
            oidc_audience,
            resource_url,
            allowed_origins,
            allowed_hosts,
            timeout,
        })
    }
}

impl Config {
    /// This configuration as the `OidcConfig` token validation expects.
    ///
    /// Built through `OidcConfig::from_map` rather than by hand so the MCP
    /// server and `RDataCore` cannot end up with differently-validated
    /// configurations of the same thing.
    #[must_use]
    pub fn oidc_config(&self) -> Option<r_data_core_core::oidc::OidcConfig> {
        let (Some(issuer), Some(audience)) = (&self.oidc_issuer, &self.oidc_audience) else {
            return None;
        };
        let pairs: HashMap<String, String> = [
            ("RDC_OIDC_ISSUER".to_string(), issuer.clone()),
            ("RDC_OIDC_AUDIENCE".to_string(), audience.clone()),
        ]
        .into_iter()
        .collect();
        r_data_core_core::oidc::OidcConfig::from_map(&pairs)
            .ok()
            .flatten()
    }
}

/// Split a comma-separated list, discarding empties.
fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

/// The `scheme://host[:port]` of a URL.
fn origin_of(raw: Option<&str>) -> Option<String> {
    let parsed = Url::parse(raw?).ok()?;
    let host = parsed.host_str()?;
    let scheme = parsed.scheme();
    Some(parsed.port().map_or_else(
        || format!("{scheme}://{host}"),
        |port| format!("{scheme}://{host}:{port}"),
    ))
}

/// The `host[:port]` of a URL, for `Host` validation.
fn authority_of(raw: Option<&str>) -> Option<String> {
    let parsed = Url::parse(raw?).ok()?;
    let host = parsed.host_str()?;
    Some(
        parsed
            .port()
            .map_or_else(|| host.to_string(), |port| format!("{host}:{port}")),
    )
}

/// Fetch a variable, treating an empty value as absent.
///
/// An empty string in a compose file or CI secret is a far more common way to
/// "not set" something than removing the line, and treating `FOO=` as a
/// configured empty value produces baffling failures.
fn get<'a>(map: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    map.get(key).map(String::as_str).filter(|v| !v.is_empty())
}

fn validate_resource_url(raw: Option<&str>) -> Result<(), ConfigError> {
    let raw = raw.ok_or(ConfigError::MissingResourceUrl)?;
    let parsed = Url::parse(raw).map_err(|e| ConfigError::InvalidResourceUrl(e.to_string()))?;
    let is_local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if parsed.scheme() == "https" || is_local {
        Ok(())
    } else {
        Err(ConfigError::InsecureResourceUrl)
    }
}

#[cfg(test)]
mod tests;
