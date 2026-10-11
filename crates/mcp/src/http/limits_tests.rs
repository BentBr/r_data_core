#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Connection limits: header timeout, first-byte wait and the connection cap.
//!
//! Real sockets against a real listener; these are properties of the accept
//! loop, not of any function it calls.

use super::http_tests::{client, start};
use super::*;

#[tokio::test]
async fn a_client_that_never_finishes_its_headers_is_disconnected() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let server = start(&[("RDC_MCP_HEADER_TIMEOUT_SECS", "1")]).await;
    let addr = server.base.trim_start_matches("http://").to_string();

    let mut socket = tokio::net::TcpStream::connect(&addr)
        .await
        .expect("connect");
    // A request line and one header, then silence: the slowloris shape.
    socket
        .write_all(b"POST /mcp HTTP/1.1\r\nHost: localhost\r\n")
        .await
        .expect("write");

    let started = std::time::Instant::now();
    let mut buf = [0u8; 512];
    // hyper answers 408 before closing, or just closes; either way the read
    // must end. Five seconds is the ceiling the test tolerates, not the
    // behaviour it expects — that is one second.
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match socket.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
    .await;

    assert!(
        outcome.is_ok(),
        "the server kept a half-sent request open for more than 5 s"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "closed, but only after {:?}; the 1 s header timeout is not in effect",
        started.elapsed()
    );
}

#[tokio::test]
async fn connections_beyond_the_cap_are_refused_until_one_closes() {
    use tokio::io::AsyncWriteExt as _;

    let server = start(&[
        ("RDC_MCP_MAX_CONNECTIONS", "1"),
        ("RDC_MCP_HEADER_TIMEOUT_SECS", "30"),
    ])
    .await;
    let addr = server.base.trim_start_matches("http://").to_string();

    // Occupy the only slot with a connection that never completes.
    let mut holder = tokio::net::TcpStream::connect(&addr)
        .await
        .expect("connect");
    holder
        .write_all(b"GET /health HTTP/1.1\r\n")
        .await
        .expect("write");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let refused = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .expect("client")
        .get(format!("{}{HEALTH_PATH}", server.base))
        .send()
        .await;
    assert!(
        refused.is_err(),
        "a second connection was served although the cap is 1"
    );

    // Release the slot; the next request must get through.
    drop(holder);
    let mut final_status = None;
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Ok(response) = client()
            .get(format!("{}{HEALTH_PATH}", server.base))
            .send()
            .await
        {
            final_status = Some(response.status());
            break;
        }
    }
    assert_eq!(
        final_status,
        Some(reqwest::StatusCode::OK),
        "the slot was not released"
    );
}

#[tokio::test]
async fn a_client_that_connects_and_sends_nothing_is_disconnected() {
    use tokio::io::AsyncReadExt as _;

    let server = start(&[("RDC_MCP_HEADER_TIMEOUT_SECS", "1")]).await;
    let addr = server.base.trim_start_matches("http://").to_string();

    // No bytes at all: hyper's header timer does not start until the first
    // byte arrives, so this is bounded by the first-byte wait, not by hyper.
    let mut socket = tokio::net::TcpStream::connect(&addr)
        .await
        .expect("connect");

    let started = std::time::Instant::now();
    let mut buf = [0u8; 512];
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match socket.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
    .await;

    assert!(
        outcome.is_ok(),
        "the server kept a silent connection open for more than 5 s"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "closed, but only after {:?}; the 1 s first-byte bound is not in effect",
        started.elapsed()
    );
}

#[tokio::test]
async fn the_permissions_cache_never_grows_past_its_capacity() {
    let pairs: std::collections::HashMap<String, String> = [
        // Unreachable on purpose: a failed lookup still caches an empty set,
        // which is what the cap has to bound.
        ("RDC_BASE_URL", "http://127.0.0.1:9"),
        ("RDC_MCP_TRANSPORT", "http"),
        ("RDC_MCP_RESOURCE_URL", "https://mcp.example.com"),
        ("RDC_OIDC_ISSUER", "https://auth.example.com"),
        ("RDC_OIDC_AUDIENCE", "rdc-mcp"),
        ("RDC_MCP_TIMEOUT_SECS", "1"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();
    let state = ServerState::new(Config::from_map(&pairs).expect("valid"))
        .expect("state")
        .with_permissions_capacity(3);

    for caller in ["one", "two", "three", "four"] {
        state
            .permissions_for(&CallerContext {
                bearer: Some(caller.to_string()),
                validated: true,
            })
            .await;
    }

    assert_eq!(state.permissions_cache_len().await, 3);
}
