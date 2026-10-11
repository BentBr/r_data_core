#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The accept loop.

use std::convert::Infallible;
use std::sync::Arc;

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
                async move { Ok::<_, Infallible>(routes::route(state, req).await) }
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
