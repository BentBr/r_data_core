use crate::data::adapters::auth::{AuthConfig, KeyLocation};
use crate::dsl::to::validate_auth_config;
use std::collections::HashMap;

#[test]
fn auth_none_is_always_ok() {
    assert!(validate_auth_config(0, &AuthConfig::None, "to").is_ok());
}

#[test]
fn auth_api_key_valid_ok() {
    let auth = AuthConfig::ApiKey {
        key: "secret".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_ok());
}

#[test]
fn auth_api_key_empty_key_fails() {
    let auth = AuthConfig::ApiKey {
        key: String::new(),
        header_name: "X-API-Key".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_api_key_empty_header_name_fails() {
    let auth = AuthConfig::ApiKey {
        key: "secret".to_string(),
        header_name: "  ".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_basic_valid_ok() {
    let auth = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_ok());
}

#[test]
fn auth_basic_empty_username_fails() {
    let auth = AuthConfig::BasicAuth {
        username: String::new(),
        password: "pass".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_basic_empty_password_fails() {
    let auth = AuthConfig::BasicAuth {
        username: "user".to_string(),
        password: "   ".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_pre_shared_key_valid_ok() {
    let auth = AuthConfig::PreSharedKey {
        key: "shared-secret".to_string(),
        location: KeyLocation::Header,
        field_name: "X-PSK".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_ok());
}

#[test]
fn auth_pre_shared_key_empty_key_fails() {
    let auth = AuthConfig::PreSharedKey {
        key: String::new(),
        location: KeyLocation::Body,
        field_name: "psk".to_string(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_pre_shared_key_empty_field_name_fails() {
    let auth = AuthConfig::PreSharedKey {
        key: "secret".to_string(),
        location: KeyLocation::Body,
        field_name: String::new(),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_entity_jwt_none_claims_ok() {
    let auth = AuthConfig::EntityJwt {
        required_claims: None,
    };
    assert!(validate_auth_config(0, &auth, "to").is_ok());
}

#[test]
fn auth_entity_jwt_empty_claim_key_fails() {
    let mut claims = HashMap::new();
    claims.insert(String::new(), serde_json::Value::String("x".to_string()));
    let auth = AuthConfig::EntityJwt {
        required_claims: Some(claims),
    };
    assert!(validate_auth_config(0, &auth, "to").is_err());
}

#[test]
fn auth_entity_jwt_valid_claims_ok() {
    let mut claims = HashMap::new();
    claims.insert(
        "role".to_string(),
        serde_json::Value::String("admin".to_string()),
    );
    let auth = AuthConfig::EntityJwt {
        required_claims: Some(claims),
    };
    assert!(validate_auth_config(0, &auth, "to").is_ok());
}
