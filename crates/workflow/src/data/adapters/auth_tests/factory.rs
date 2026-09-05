use super::*;

#[test]
fn create_auth_provider_none_variant() {
    let provider = create_auth_provider(&AuthConfig::None).unwrap();
    assert_eq!(provider.auth_type(), "none");
}

#[test]
fn create_auth_provider_api_key_variant() {
    let config = AuthConfig::ApiKey {
        key: "k".to_string(),
        header_name: "X-Key".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "api_key");
}

#[test]
fn create_auth_provider_basic_auth_variant() {
    let config = AuthConfig::BasicAuth {
        username: "u".to_string(),
        password: "p".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "basic_auth");
}

#[test]
fn create_auth_provider_pre_shared_key_variant() {
    let config = AuthConfig::PreSharedKey {
        key: "k".to_string(),
        location: KeyLocation::Header,
        field_name: "X-PSK".to_string(),
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "pre_shared_key");
}

#[test]
fn create_auth_provider_entity_jwt_variant() {
    let config = AuthConfig::EntityJwt {
        required_claims: None,
    };
    let provider = create_auth_provider(&config).unwrap();
    assert_eq!(provider.auth_type(), "entity_jwt");
}
