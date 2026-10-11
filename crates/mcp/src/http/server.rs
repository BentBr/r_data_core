#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The accept loop.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::UpgradeableConnection;
use tokio::sync::Semaphore;

pub use crate::config::{
    HttpServerLimits, DEFAULT_HEADER_READ_TIMEOUT_SECS, DEFAULT_IDLE_TIMEOUT_SECS,
    DEFAULT_MAX_CONNECTIONS,
};

use super::{routes, ServerState};

mod lifetime;

use lifetime::{run_bounded, Ended};

/// How long a connection asked to close at its deadline may take to finish
/// the request it is serving before it is cut.
pub const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);
/// HTTP/2 keep-alive: how long a ping may go unanswered before the
/// connection is considered dead.
const HTTP2_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);
/// Pause after a failed `accept`. The usual cause is running out of file
/// descriptors, and retrying at once only spins the CPU until one frees up.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

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
                tokio::time::sleep(ACCEPT_BACKOFF).await;
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
                .keep_alive_timeout(HTTP2_KEEP_ALIVE_TIMEOUT);

            let connection = builder.serve_connection_with_upgrades(TokioIo::new(stream), service);

            // At the deadline the connection is asked to close rather than
            // dropped: hyper finishes the request in flight, answers it, and
            // then closes. Only a request still running after the grace
            // period is cut.
            match run_bounded(
                connection,
                limits.idle_timeout,
                SHUTDOWN_GRACE,
                UpgradeableConnection::graceful_shutdown,
            )
            .await
            {
                Ended::Finished(Ok(())) | Ended::Drained(Ok(())) => {}
                Ended::Finished(Err(e)) | Ended::Drained(Err(e)) => {
                    log::debug!("connection from {peer} ended: {e}");
                }
                Ended::Cut => log::debug!(
                    "connection from {peer} cut: still busy {SHUTDOWN_GRACE:?} after its {:?} limit",
                    limits.idle_timeout
                ),
            }
        });
    }
}
