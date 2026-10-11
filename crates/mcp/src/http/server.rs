#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The accept loop.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::sync::Semaphore;

use crate::config::{positive_u64, ConfigError};

use super::{routes, ServerState};

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
/// `idle_timeout` bounds the whole connection, not the gap between requests:
/// hyper 1 has no idle timer, so the connection future is wrapped in a timeout.
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

/// Serve until `shutdown` fires.
///
/// # Errors
/// Returns an error if the listener cannot be bound.
pub async fn serve(
    state: Arc<ServerState>,
    listener: tokio::net::TcpListener,
    shutdown: tokio_util::sync::CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limits = state.limits().clone();
    let slots = Arc::new(Semaphore::new(limits.max_connections));

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

        // Refuse rather than queue. A queued connection still holds a socket,
        // which is the resource the cap exists to protect; a refused one
        // costs nothing and the client sees a reset it can retry.
        let Ok(permit) = Arc::clone(&slots).try_acquire_owned() else {
            log::warn!(
                "refusing a connection from {peer}: {} connections already open",
                limits.max_connections
            );
            drop(stream);
            continue;
        };

        let state = Arc::clone(&state);
        let limits = limits.clone();
        tokio::spawn(async move {
            // Held for the life of the connection; dropped with the task.
            let _permit = permit;

            // The auto builder sniffs the HTTP version from the first bytes
            // before hyper's header timer starts, so a client that sends
            // nothing would otherwise hold its slot for `idle_timeout`.
            // `peek` leaves the byte for hyper to read.
            match tokio::time::timeout(limits.header_read_timeout, stream.peek(&mut [0u8; 1])).await
            {
                Ok(Ok(n)) if n > 0 => {}
                Ok(_) => {
                    log::debug!("connection from {peer} closed before sending anything");
                    return;
                }
                Err(_) => {
                    log::debug!(
                        "connection from {peer} sent nothing within {:?}",
                        limits.header_read_timeout
                    );
                    return;
                }
            }

            let service = hyper::service::service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { Ok::<_, Infallible>(routes::route(state, req).await) }
            });

            let mut builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
            // The timer is what makes the header timeout real: hyper's default
            // of 30 s is silently inert without one.
            builder
                .http1()
                .timer(TokioTimer::new())
                .header_read_timeout(limits.header_read_timeout);
            builder
                .http2()
                .timer(TokioTimer::new())
                .keep_alive_interval(Some(Duration::from_secs(30)))
                .keep_alive_timeout(limits.idle_timeout);

            let connection = builder.serve_connection_with_upgrades(TokioIo::new(stream), service);

            match tokio::time::timeout(limits.idle_timeout, connection).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => log::debug!("connection from {peer} ended: {e}"),
                Err(_) => log::debug!(
                    "connection from {peer} closed after {:?}",
                    limits.idle_timeout
                ),
            }
        });
    }
}
