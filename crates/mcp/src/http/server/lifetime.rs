#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Bounding a connection's lifetime without cutting a request in half.
//!
//! Wrapping the connection future in a plain timeout drops it at the deadline,
//! which aborts whatever request it is serving: the client gets a reset after
//! the server may already have acted on the request. So the deadline asks the
//! connection to shut down gracefully instead — no new requests, finish the
//! current one, then close — and a second, short timer cuts it only if that
//! request is still running after the grace period.

use std::future::Future;
use std::pin::{pin, Pin};
use std::time::Duration;

/// How a bounded connection ended.
#[derive(Debug)]
pub enum Ended<E> {
    /// Closed on its own before the deadline.
    Finished(Result<(), E>),
    /// Asked to close at the deadline, and did within the grace period.
    Drained(Result<(), E>),
    /// Still busy at the end of the grace period, and dropped.
    Cut,
}

/// Drive `connection` for at most `lifetime`, then call `shutdown` on it and
/// give it `grace` more to finish.
///
/// `shutdown` is hyper's `graceful_shutdown`, passed in so this sequencing can
/// be tested on its own.
pub async fn run_bounded<F, E, S>(
    connection: F,
    lifetime: Duration,
    grace: Duration,
    shutdown: S,
) -> Ended<E>
where
    F: Future<Output = Result<(), E>>,
    S: FnOnce(Pin<&mut F>),
{
    let mut connection = pin!(connection);
    if let Ok(result) = tokio::time::timeout(lifetime, connection.as_mut()).await {
        return Ended::Finished(result);
    }
    shutdown(connection.as_mut());
    tokio::time::timeout(grace, connection)
        .await
        .map_or(Ended::Cut, Ended::Drained)
}

#[cfg(test)]
mod lifetime_tests;
