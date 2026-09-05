use super::*;

fn builder() -> reqwest::RequestBuilder {
    reqwest::Client::new().get("http://example.com/resource")
}

#[test]
fn no_auth_provider_leaves_request_unchanged() {
    let provider = NoAuthProvider;
    assert_eq!(provider.auth_type(), "none");

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert!(req.headers().get("Authorization").is_none());
}

#[test]
fn no_auth_provider_extracts_nothing() {
    let provider = NoAuthProvider;
    let req = actix_web::test::TestRequest::default().to_http_request();
    assert_eq!(provider.extract_from_request(&req).unwrap(), None);
}

#[test]
fn api_key_provider_sets_configured_header() {
    let provider = ApiKeyAuthProvider::new("secret-key".to_string(), Some("X-Custom".to_string()));
    assert_eq!(provider.auth_type(), "api_key");

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(
        req.headers().get("X-Custom").and_then(|v| v.to_str().ok()),
        Some("secret-key")
    );
}

#[test]
fn api_key_provider_defaults_header_name_when_none_given() {
    let provider = ApiKeyAuthProvider::new("secret-key".to_string(), None);
    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(
        req.headers().get("X-API-Key").and_then(|v| v.to_str().ok()),
        Some("secret-key")
    );
}

#[test]
fn basic_auth_provider_sets_authorization_header() {
    let provider = BasicAuthProvider::new("alice".to_string(), "hunter2".to_string());
    assert_eq!(provider.auth_type(), "basic_auth");

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(auth_header.starts_with("Basic "));
}

#[test]
fn pre_shared_key_header_location_sets_header() {
    let provider = PreSharedKeyAuthProvider::new(
        "shared-secret".to_string(),
        KeyLocation::Header,
        "X-PSK".to_string(),
    );
    assert_eq!(provider.auth_type(), "pre_shared_key");

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(
        req.headers().get("X-PSK").and_then(|v| v.to_str().ok()),
        Some("shared-secret")
    );
}

#[test]
fn pre_shared_key_body_location_does_not_touch_headers() {
    let provider = PreSharedKeyAuthProvider::new(
        "shared-secret".to_string(),
        KeyLocation::Body,
        "psk".to_string(),
    );

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert!(req.headers().get("psk").is_none());
    assert!(req.headers().get("shared-secret").is_none());
}

#[test]
fn entity_jwt_provider_does_not_modify_outgoing_request() {
    let provider = EntityJwtAuthProvider::new(None);
    assert_eq!(provider.auth_type(), "entity_jwt");

    let req = provider
        .apply_to_request(builder())
        .unwrap()
        .build()
        .unwrap();
    assert!(req.headers().get("Authorization").is_none());
}

#[test]
fn entity_jwt_provider_exposes_required_claims() {
    let mut claims = HashMap::new();
    claims.insert(
        "role".to_string(),
        serde_json::Value::String("admin".to_string()),
    );
    let provider = EntityJwtAuthProvider::new(Some(claims.clone()));
    assert_eq!(provider.required_claims(), &Some(claims));
}

#[test]
fn entity_jwt_provider_required_claims_none_by_default() {
    let provider = EntityJwtAuthProvider::new(None);
    assert!(provider.required_claims().is_none());
}
