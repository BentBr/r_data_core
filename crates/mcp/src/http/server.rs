#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The accept loop.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::sync::Semaphore;

pub use crate::config::{
    HttpServerLimits, DEFAULT_HEADER_READ_TIMEOUT_SECS, DEFAULT_IDLE_TIMEOUT_SECS,
    DEFAULT_MAX_CONNECTIONS,
};

use super::{routes, ServerState};

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
