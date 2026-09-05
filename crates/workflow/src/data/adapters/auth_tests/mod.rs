#![allow(clippy::unwrap_used)]

mod extraction;
mod factory;
mod providers;

use super::*;

#[test]
fn test_entity_jwt_config_serialization() {
    let config = AuthConfig::EntityJwt {
        required_claims: None,
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "entity_jwt");

    let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
    match deserialized {
        AuthConfig::EntityJwt { required_claims } => {
            assert!(required_claims.is_none());
        }
        AuthConfig::None
        | AuthConfig::ApiKey { .. }
        | AuthConfig::BasicAuth { .. }
        | AuthConfig::PreSharedKey { .. } => panic!("Expected EntityJwt variant"),
    }
}

#[test]
fn test_entity_jwt_config_with_required_claims() {
    let mut claims = HashMap::new();
    claims.insert(
        "extra.role".to_string(),
        serde_json::Value::String("admin".to_string()),
    );
    let config = AuthConfig::EntityJwt {
        required_claims: Some(claims),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "entity_jwt");
    assert_eq!(json["required_claims"]["extra.role"], "admin");

    let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
    match deserialized {
        AuthConfig::EntityJwt { required_claims } => {
            let claims = required_claims.unwrap();
            assert_eq!(
                claims.get("extra.role"),
                Some(&serde_json::Value::String("admin".to_string()))
            );
        }
        AuthConfig::None
        | AuthConfig::ApiKey { .. }
        | AuthConfig::BasicAuth { .. }
        | AuthConfig::PreSharedKey { .. } => panic!("Expected EntityJwt variant"),
    }
}

#[test]
fn test_entity_jwt_config_without_required_claims_is_null() {
    let config = AuthConfig::EntityJwt {
        required_claims: None,
    };
    let json = serde_json::to_value(&config).unwrap();
    // required_claims is null when None
    assert_eq!(json["required_claims"], serde_json::Value::Null);
}

#[test]
fn key_location_serde_tags_are_snake_case() {
    assert_eq!(
        serde_json::to_value(KeyLocation::Header).unwrap(),
        serde_json::json!("header")
    );
    assert_eq!(
        serde_json::to_value(KeyLocation::Body).unwrap(),
        serde_json::json!("body")
    );
}

#[test]
fn api_key_header_name_defaults_when_omitted() {
    let json = serde_json::json!({ "type": "api_key", "key": "secret" });
    let config: AuthConfig = serde_json::from_value(json).unwrap();
    match config {
        AuthConfig::ApiKey { key, header_name } => {
            assert_eq!(key, "secret");
            assert_eq!(header_name, "X-API-Key");
        }
        AuthConfig::None
        | AuthConfig::BasicAuth { .. }
        | AuthConfig::PreSharedKey { .. }
        | AuthConfig::EntityJwt { .. } => panic!("Expected ApiKey variant"),
    }
}

#[test]
fn basic_auth_round_trips_through_json() {
    let config = AuthConfig::BasicAuth {
        username: "alice".to_string(),
        password: "hunter2".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "basic_auth");
    let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
    match deserialized {
        AuthConfig::BasicAuth { username, password } => {
            assert_eq!(username, "alice");
            assert_eq!(password, "hunter2");
        }
        AuthConfig::None
        | AuthConfig::ApiKey { .. }
        | AuthConfig::PreSharedKey { .. }
        | AuthConfig::EntityJwt { .. } => panic!("Expected BasicAuth variant"),
    }
}

#[test]
fn pre_shared_key_round_trips_with_location() {
    let config = AuthConfig::PreSharedKey {
        key: "shared".to_string(),
        location: KeyLocation::Header,
        field_name: "X-PSK".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["type"], "pre_shared_key");
    assert_eq!(json["location"], "header");
    let deserialized: AuthConfig = serde_json::from_value(json).unwrap();
    match deserialized {
        AuthConfig::PreSharedKey {
            key,
            location,
            field_name,
        } => {
            assert_eq!(key, "shared");
            assert!(matches!(location, KeyLocation::Header));
            assert_eq!(field_name, "X-PSK");
        }
        AuthConfig::None
        | AuthConfig::ApiKey { .. }
        | AuthConfig::BasicAuth { .. }
        | AuthConfig::EntityJwt { .. } => panic!("Expected PreSharedKey variant"),
    }
}
