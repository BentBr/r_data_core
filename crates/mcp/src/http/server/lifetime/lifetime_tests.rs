#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! The deadline drains a request in flight instead of aborting it.
//!
//! Real hyper connections over real sockets, with a handler slow enough to be
//! running when the deadline passes. Times are short but the margins between
//! them are wide, so scheduling jitter cannot flip an outcome.

use std::convert::Infallible;
use std::time::{Duration, Instant};

use http_body_util::Full;
use hyper::body::Bytes;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto::UpgradeableConnection;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::{run_bounded, Ended};

/// Serve one connection whose handler takes `handler_delay`, bounded by
/// `lifetime` and `grace`; returns the client socket and how it ended.
async fn bounded_connection(
    lifetime: Duration,
    grace: Duration,
    handler_delay: Duration,
) -> (
    tokio::net::TcpStream,
    tokio::task::JoinHandle<Ended<Box<dyn std::error::Error + Send + Sync>>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let service = hyper::service::service_fn(move |_req| async move {
            tokio::time::sleep(handler_delay).await;
            Ok::<_, Infallible>(hyper::Response::new(Full::new(Bytes::from_static(b"done"))))
        });
        let mut builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
        builder.http1().timer(TokioTimer::new());
        let connection = builder.serve_connection_with_upgrades(TokioIo::new(stream), service);
        run_bounded(
            connection,
            lifetime,
            grace,
            UpgradeableConnection::graceful_shutdown,
        )
        .await
    });
    let client = tokio::net::TcpStream::connect(addr).await.expect("connect");
    (client, server)
}

/// Read until the server closes, or give up after `limit`.
async fn read_to_close(socket: &mut tokio::net::TcpStream, limit: Duration) -> String {
    let mut out = Vec::new();
    let _ = tokio::time::timeout(limit, socket.read_to_end(&mut out)).await;
    String::from_utf8_lossy(&out).into_owned()
}

const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";

#[tokio::test]
async fn a_request_in_flight_at_the_deadline_still_gets_its_response() {
    // Deadline at 500 ms; the request starts at once and needs 1.5 s, so it
    // is mid-flight when the deadline passes and finishes well inside grace.
    let (mut client, server) = bounded_connection(
        Duration::from_millis(500),
        Duration::from_secs(5),
        Duration::from_millis(1500),
    )
    .await;
    client.write_all(REQUEST).await.expect("write");

    let response = read_to_close(&mut client, Duration::from_secs(10)).await;
    assert!(response.starts_with("HTTP/1.1 200"), "got: {response:?}");
    assert!(response.ends_with("done"), "got: {response:?}");

    let ended = server.await.expect("server task");
    assert!(matches!(ended, Ended::Drained(Ok(()))), "{ended:?}");
}

#[tokio::test]
async fn a_request_still_running_after_the_grace_period_is_cut() {
    let started = Instant::now();
    let (mut client, server) = bounded_connection(
        Duration::from_millis(300),
        Duration::from_millis(300),
        Duration::from_secs(30),
    )
    .await;
    client.write_all(REQUEST).await.expect("write");

    let ended = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("the connection outlived lifetime + grace")
        .expect("server task");
    assert!(matches!(ended, Ended::Cut), "{ended:?}");
    assert!(started.elapsed() < Duration::from_secs(5));

    let response = read_to_close(&mut client, Duration::from_secs(2)).await;
    assert!(
        response.is_empty(),
        "a cut request has no response: {response:?}"
    );
}

#[tokio::test]
async fn an_idle_connection_closes_at_the_deadline() {
    let started = Instant::now();
    let (mut client, server) = bounded_connection(
        Duration::from_millis(300),
        Duration::from_secs(5),
        Duration::ZERO,
    )
    .await;
    // One quick request, then nothing: keep-alive would otherwise hold the
    // connection open indefinitely.
    client.write_all(REQUEST).await.expect("write");

    let response = read_to_close(&mut client, Duration::from_secs(5)).await;
    assert!(response.starts_with("HTTP/1.1 200"), "got: {response:?}");
    let ended = server.await.expect("server task");
    assert!(matches!(ended, Ended::Drained(Ok(()))), "{ended:?}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "an idle connection waited out the grace period: {:?}",
        started.elapsed()
    );
}
