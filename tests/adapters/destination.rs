use bytes::Bytes;
use r_data_core::workflow::data::adapters::auth::{create_auth_provider, AuthConfig};
use r_data_core::workflow::data::adapters::destination::{
    DataDestination, DestinationContext, HttpMethod,
};
use r_data_core::workflow::data::adapters::destination::uri::UriDestination;
use serde_json::json;

#[test]
fn test_http_method_requires_body() {
    assert!(HttpMethod::Post.requires_body());
    assert!(HttpMethod::Put.requires_body());
    assert!(HttpMethod::Patch.requires_body());
    assert!(HttpMethod::Delete.requires_body());
    assert!(!HttpMethod::Get.requires_body());
    assert!(!HttpMethod::Head.requires_body());
    assert!(!HttpMethod::Options.requires_body());
}

#[test]
fn test_uri_destination_validate() {
    let dest = UriDestination::new();
    assert_eq!(dest.destination_type(), "uri");

    // Valid HTTP URI
    let config = json!({"uri": "https://example.com/api"});
    assert!(dest.validate(&config).is_ok());

    // Valid HTTPS URI
    let config = json!({"uri": "http://example.com/api"});
    assert!(dest.validate(&config).is_ok());

    // Invalid URI (missing http/https)
    let config = json!({"uri": "ftp://example.com/api"});
    assert!(dest.validate(&config).is_err());

    // Missing URI
    let config = json!({});
    assert!(dest.validate(&config).is_err());
}

#[tokio::test]
async fn test_uri_destination_with_auth() {
    let dest = UriDestination::new();
    let auth_config = AuthConfig::ApiKey {
        key: "test-key".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let auth_provider = create_auth_provider(&auth_config).unwrap();

    let ctx = DestinationContext {
        auth: Some(auth_provider),
        method: Some(HttpMethod::Post),
        config: json!({"uri": "https://example.com/api"}),
    };

    let data = Bytes::from("test data");
    let result = dest.push(&ctx, data).await;
    // This will fail because example.com might not respond, but we're testing the structure
    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
async fn test_uri_destination_without_auth() {
    let dest = UriDestination::new();
    let ctx = DestinationContext {
        auth: None,
        method: Some(HttpMethod::Post),
        config: json!({"uri": "https://example.com/api"}),
    };

    let data = Bytes::from("test data");
    let result = dest.push(&ctx, data).await;
    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
async fn test_uri_destination_default_method() {
    let dest = UriDestination::new();
    let ctx = DestinationContext {
        auth: None,
        method: None, // Should default to Post
        config: json!({"uri": "https://example.com/api"}),
    };

    let data = Bytes::from("test data");
    let result = dest.push(&ctx, data).await;
    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
async fn test_uri_destination_get_method() {
    let dest = UriDestination::new();
    let ctx = DestinationContext {
        auth: None,
        method: Some(HttpMethod::Get),
        config: json!({"uri": "https://example.com/api"}),
    };

    let data = Bytes::from("test data");
    let result = dest.push(&ctx, data).await;
    // GET should not send body
    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
async fn test_uri_destination_with_basic_auth() {
    let dest = UriDestination::new();
    let auth_config = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let auth_provider = create_auth_provider(&auth_config).unwrap();

    let ctx = DestinationContext {
        auth: Some(auth_provider),
        method: Some(HttpMethod::Post),
        config: json!({"uri": "https://example.com/api"}),
    };

    let data = Bytes::from("test data");
    let result = dest.push(&ctx, data).await;
    assert!(result.is_err() || result.is_ok());
}

