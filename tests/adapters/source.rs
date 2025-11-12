use r_data_core::workflow::data::adapters::auth::{create_auth_provider, AuthConfig};
use r_data_core::workflow::data::adapters::source::uri::UriSource;
use r_data_core::workflow::data::adapters::source::{DataSource, SourceContext};
use serde_json::json;

#[tokio::test]
async fn test_uri_source_validate() {
    let source = UriSource::new();
    assert_eq!(source.source_type(), "uri");

    // Valid HTTP URI
    let config = json!({"uri": "https://example.com/data"});
    assert!(source.validate(&config).is_ok());

    // Valid HTTPS URI
    let config = json!({"uri": "http://example.com/data"});
    assert!(source.validate(&config).is_ok());

    // Invalid URI (missing http/https)
    let config = json!({"uri": "ftp://example.com/data"});
    assert!(source.validate(&config).is_err());

    // Missing URI
    let config = json!({});
    assert!(source.validate(&config).is_err());
}

#[tokio::test]
async fn test_uri_source_with_auth() {
    use futures::StreamExt;
    use httpmock::{Method::GET, MockServer};

    // Start a local mock server
    let server = MockServer::start_async().await;

    // Expect GET /data with API key header and return a small body
    let mock = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/data")
                .header("X-API-Key", "test-key");
            then.status(200).body("hello");
        })
        .await;

    let source = UriSource::new();
    let auth_config = AuthConfig::ApiKey {
        key: "test-key".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let auth_provider = create_auth_provider(&auth_config).unwrap();

    let ctx = SourceContext {
        auth: Some(auth_provider),
        config: json!({ "uri": server.url("/data") }),
    };

    // Perform fetch against mock server
    let mut stream = source.fetch(&ctx).await.expect("fetch should succeed");

    // Read the single chunk and verify content
    let first = stream.next().await.expect("one chunk expected").unwrap();
    assert_eq!(&first[..], b"hello");

    // Verify mock was actually hit
    mock.assert_async().await;
}

#[tokio::test]
async fn test_uri_source_without_auth() {
    use futures::StreamExt;
    use httpmock::{Method::GET, MockServer};

    // Local mock server without any auth requirements
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/data");
            then.status(200).body("ok");
        })
        .await;

    let source = UriSource::new();
    let ctx = SourceContext {
        auth: None,
        config: json!({ "uri": server.url("/data") }),
    };

    // Should fetch from local server quickly
    let mut stream = source.fetch(&ctx).await.expect("fetch should succeed");
    let first = stream.next().await.expect("one chunk expected").unwrap();
    assert_eq!(&first[..], b"ok");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_uri_source_with_basic_auth() {
    use futures::StreamExt;
    use httpmock::{Method::GET, MockServer};

    // Authorization: Basic base64("user:pass") == "Basic dXNlcjpwYXNz"
    let server = MockServer::start_async().await;
    let mock = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/data")
                .header("Authorization", "Basic dXNlcjpwYXNz");
            then.status(200).body("ok");
        })
        .await;

    let source = UriSource::new();
    let auth_config = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let auth_provider = create_auth_provider(&auth_config).unwrap();

    let ctx = SourceContext {
        auth: Some(auth_provider),
        config: json!({ "uri": server.url("/data") }),
    };

    let mut stream = source.fetch(&ctx).await.expect("fetch should succeed");
    let first = stream.next().await.expect("one chunk expected").unwrap();
    assert_eq!(&first[..], b"ok");

    mock.assert_async().await;
}
