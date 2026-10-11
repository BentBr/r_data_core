#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The HTTP accept loop's limits, as configuration.
//!
//! Lives here rather than beside the accept loop so that `config` does not
//! import from `http`: the loop reads these, configuration does not read the
//! loop. `http::server` re-exports the type for its callers.

use std::collections::HashMap;
use std::time::Duration;

use super::{positive_u64, ConfigError};

/// Seconds a client gets to finish sending request headers.
pub const DEFAULT_HEADER_READ_TIMEOUT_SECS: u64 = 10;
/// Seconds a connection may stay open in total.
///
/// A bound on the connection, not on idle time between requests: hyper 1 has
/// no idle timer, so the whole connection future is bounded instead. Above the
/// longest tool call (`run_workflow` waits at most 55 s) by a wide margin.
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 120;
/// Connections accepted at once; the rest are refused immediately.
pub const DEFAULT_MAX_CONNECTIONS: usize = 1024;

/// What keeps one slow or hostile client from holding the server.
///
/// hyper's own 30 s header timeout is inert unless a `Timer` is installed —
/// which the builder does not do by itself — so without this a connection
/// that sends half a request line holds its task forever.
///
/// `header_read_timeout` also bounds the time to the first byte: a client that
/// connects and sends nothing is dropped after the same interval.
///
/// `idle_timeout` bounds the whole connection, not the gap between requests;
/// see [`DEFAULT_IDLE_TIMEOUT_SECS`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpServerLimits {
    pub header_read_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_connections: usize,
}

impl Default for HttpServerLimits {
    fn default() -> Self {
        Self {
            header_read_timeout: Duration::from_secs(DEFAULT_HEADER_READ_TIMEOUT_SECS),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

impl HttpServerLimits {
    /// Read the limits from an environment map.
    ///
    /// # Errors
    /// Returns `ConfigError::InvalidNumber` for a value that is not a positive
    /// integer. Zero is refused: a server with no time to read headers or no
    /// connection slots serves nothing, and should say so at startup.
    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let header = positive_u64(
            map,
            "RDC_MCP_HEADER_TIMEOUT_SECS",
            DEFAULT_HEADER_READ_TIMEOUT_SECS,
        )?;
        let idle = positive_u64(map, "RDC_MCP_IDLE_TIMEOUT_SECS", DEFAULT_IDLE_TIMEOUT_SECS)?;
        let max = positive_u64(
            map,
            "RDC_MCP_MAX_CONNECTIONS",
            DEFAULT_MAX_CONNECTIONS as u64,
        )?;
        Ok(Self {
            header_read_timeout: Duration::from_secs(header),
            idle_timeout: Duration::from_secs(idle),
            max_connections: usize::try_from(max).map_err(|_| {
                ConfigError::InvalidNumber("RDC_MCP_MAX_CONNECTIONS", max.to_string())
            })?,
        })
    }
}
