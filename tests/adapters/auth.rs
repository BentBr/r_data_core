#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_workflow::data::adapters::auth::{
    create_auth_provider, ApiKeyAuthProvider, AuthConfig, AuthProvider, BasicAuthProvider,
    KeyLocation, NoAuthProvider, PreSharedKeyAuthProvider,
};
use serde_json::json;

#[test]
fn test_no_auth_provider() {
    let provider = NoAuthProvider;
    assert_eq!(provider.auth_type(), "none");
}

#[test]
fn test_api_key_auth_provider() {
    let provider =
        ApiKeyAuthProvider::new("test-key".to_string(), Some("X-Custom-Key".to_string()));
    assert_eq!(provider.auth_type(), "api_key");

    // Test applying to request
    let client = reqwest::Client::new();
    let request = client.get("https://example.com");
    let result = provider.apply_to_request(request);
    assert!(result.is_ok());
}

#[test]
fn test_api_key_auth_provider_default_header() {
    let provider = ApiKeyAuthProvider::new("test-key".to_string(), None);
    assert_eq!(provider.auth_type(), "api_key");

    let client = reqwest::Client::new();
    let request = client.get("https://example.com");
    let result = provider.apply_to_request(request);
    assert!(result.is_ok());
}

#[test]
fn test_basic_auth_provider() {
    let provider = BasicAuthProvider::new("user".to_string(), "pass".to_string());
    assert_eq!(provider.auth_type(), "basic_auth");

    let client = reqwest::Client::new();
    let request = client.get("https://example.com");
    let result = provider.apply_to_request(request);
    assert!(result.is_ok());
}

#[test]
fn test_pre_shared_key_auth_provider_header() {
    let provider = PreSharedKeyAuthProvider::new(
        "secret-key".to_string(),
        KeyLocation::Header,
        "X-Pre-Shared-Key".to_string(),
    );
    assert_eq!(provider.auth_type(), "pre_shared_key");

    let client = reqwest::Client::new();
    let request = client.get("https://example.com");
    let result = provider.apply_to_request(request);
    assert!(result.is_ok());
}

#[test]
fn test_pre_shared_key_auth_provider_body() {
    let provider = PreSharedKeyAuthProvider::new(
        "secret-key".to_string(),
        KeyLocation::Body,
        "api_key".to_string(),
    );
    assert_eq!(provider.auth_type(), "pre_shared_key");

    // Body auth doesn't modify request builder
    let client = reqwest::Client::new();
    let request = client.get("https://example.com");
    let result = provider.apply_to_request(request);
    assert!(result.is_ok());
}

#[test]
fn test_create_auth_provider_none() {
    let config = AuthConfig::None;
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "none");
}

#[test]
fn test_create_auth_provider_api_key() {
    let config = AuthConfig::ApiKey {
        key: "test-key".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "api_key");
}

#[test]
fn test_create_auth_provider_api_key_custom_header() {
    let config = AuthConfig::ApiKey {
        key: "test-key".to_string(),
        header_name: "X-Custom-Header".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "api_key");
}

#[test]
fn test_create_auth_provider_basic_auth() {
    let config = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "basic_auth");
}

#[test]
fn test_create_auth_provider_pre_shared_key_header() {
    let config = AuthConfig::PreSharedKey {
        key: "secret".to_string(),
        location: KeyLocation::Header,
        field_name: "X-Key".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "pre_shared_key");
}

#[test]
fn test_create_auth_provider_pre_shared_key_body() {
    let config = AuthConfig::PreSharedKey {
        key: "secret".to_string(),
        location: KeyLocation::Body,
        field_name: "api_key".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "pre_shared_key");
}

#[test]
fn test_auth_config_serialization() {
    let config = AuthConfig::ApiKey {
        key: "test".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "api_key");
    assert_eq!(json["key"], "test");
    assert_eq!(json["header_name"], "X-API-Key");
}

#[test]
fn test_auth_config_deserialization() {
    let json = json!({
        "type": "api_key",
        "key": "test-key",
        "header_name": "X-Custom-Key"
    });
    let config: AuthConfig = serde_json::from_value(json).unwrap();
    match config {
        AuthConfig::ApiKey { key, header_name } => {
            assert_eq!(key, "test-key");
            assert_eq!(header_name, "X-Custom-Key");
        }
        _ => panic!("Expected ApiKey variant"),
    }
}

#[test]
fn test_auth_config_basic_auth_serialization() {
    let config = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "basic_auth");
    assert_eq!(json["username"], "user");
    assert_eq!(json["password"], "pass");
}

#[test]
fn test_auth_config_pre_shared_key_serialization() {
    let config = AuthConfig::PreSharedKey {
        key: "secret".to_string(),
        location: KeyLocation::Header,
        field_name: "X-Key".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "pre_shared_key");
    assert_eq!(json["key"], "secret");
    assert_eq!(json["location"], "header");
    assert_eq!(json["field_name"], "X-Key");
}
