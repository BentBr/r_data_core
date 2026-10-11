#![allow(clippy::expect_used, clippy::unwrap_used)]

//! How discovery fails. Each failure has to land in the right bucket:
//! `Unreachable` is the provider being unavailable (the caller sees a 503),
//! `Malformed` is the provider answering with something that must not be
//! trusted.

use std::time::{Duration, Instant};

use serde_json::json;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

async fn provider_answering(template: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(template)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_body_that_is_not_json_is_malformed() {
    let server =
        provider_answering(ResponseTemplate::new(200).set_body_string("<html>no</html>")).await;

    let error = fetch(&client().expect("client"), &server.uri(), true)
        .await
        .expect_err("refused");

    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn a_document_without_a_key_set_is_malformed() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": uri,
            "authorization_endpoint": format!("{uri}/authorize"),
            "token_endpoint": format!("{uri}/token"),
        })))
        .mount(&server)
        .await;

    let error = fetch(&client().expect("client"), &uri, true)
        .await
        .expect_err("refused");

    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn a_document_without_issuer_and_endpoints_is_malformed() {
    let server = provider_answering(ResponseTemplate::new(200).set_body_json(json!({}))).await;

    let error = fetch(&client().expect("client"), &server.uri(), true)
        .await
        .expect_err("refused");

    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
}

#[tokio::test]
async fn the_code_flow_endpoints_are_optional_in_the_document() {
    // Pinned as-is: only the key set is required to parse. The authorization
    // and token endpoints are `Option`s, checked where they are used.
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": uri,
            "jwks_uri": format!("{uri}/jwks"),
        })))
        .mount(&server)
        .await;

    let found = fetch(&client().expect("client"), &uri, true)
        .await
        .expect("accepted");

    assert!(found.authorization_endpoint.is_none());
    assert!(found.token_endpoint.is_none());
}

#[tokio::test]
async fn a_404_is_unreachable() {
    let server = provider_answering(ResponseTemplate::new(404)).await;

    let error = fetch(&client().expect("client"), &server.uri(), true)
        .await
        .expect_err("refused");

    assert!(matches!(error, KeySourceError::Unreachable(_)), "{error}");
}

#[tokio::test]
async fn a_provider_that_is_not_listening_is_unreachable() {
    // Bind to learn a free port, then close it so the connection is refused.
    let port = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let error = fetch(
        &client().expect("client"),
        &format!("http://127.0.0.1:{port}"),
        true,
    )
    .await
    .expect_err("refused");

    assert!(matches!(error, KeySourceError::Unreachable(_)), "{error}");
}

/// Serve a chunked response (so no `Content-Length`) that would take far
/// longer than the test to finish, counting the chunks actually written.
async fn endless_chunked_provider(
    chunk_count: usize,
) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let written = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = written.clone();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        // Drain the request head; its content is irrelevant.
        let mut head = [0_u8; 1024];
        let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut head).await;
        let status = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                       transfer-encoding: chunked\r\n\r\n";
        if socket.write_all(status).await.is_err() {
            return;
        }
        let payload = vec![b'x'; 64 * 1024];
        for _ in 0..chunk_count {
            let frame = [
                format!("{:x}\r\n", payload.len()).as_bytes(),
                &payload,
                b"\r\n",
            ]
            .concat();
            if socket.write_all(&frame).await.is_err() {
                return;
            }
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    (format!("http://{address}"), written)
}

#[tokio::test]
async fn an_unbounded_chunked_body_is_cut_off_at_the_cap() {
    use std::sync::atomic::Ordering;

    // 200 chunks of 64 KiB at 20 ms each is ~4 s and ~12 MiB; the cap is 1 MiB.
    let total_chunks = 200;
    let (issuer, written) = endless_chunked_provider(total_chunks).await;

    let started = Instant::now();
    let error = fetch(&client().expect("client"), &issuer, true)
        .await
        .expect_err("refused");
    let elapsed = started.elapsed();

    assert!(matches!(error, KeySourceError::Malformed(_)), "{error}");
    assert!(error.to_string().contains("exceeds"), "{error}");
    assert!(
        elapsed < Duration::from_secs(2),
        "reading must stop at the cap, took {elapsed:?}"
    );
    assert!(
        written.load(Ordering::SeqCst) < total_chunks,
        "the body was read to the end"
    );
}
