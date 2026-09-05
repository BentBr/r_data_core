use super::*;
use actix_web::test::TestRequest;

#[test]
fn pre_shared_key_header_location_extracts_present_header() {
    let provider = PreSharedKeyAuthProvider::new(
        "unused".to_string(),
        KeyLocation::Header,
        "X-PSK".to_string(),
    );
    let req = TestRequest::default()
        .insert_header(("X-PSK", "the-value"))
        .to_http_request();
    assert_eq!(
        provider.extract_from_request(&req).unwrap(),
        Some("the-value".to_string())
    );
}

#[test]
fn pre_shared_key_header_location_missing_header_returns_none() {
    let provider = PreSharedKeyAuthProvider::new(
        "unused".to_string(),
        KeyLocation::Header,
        "X-PSK".to_string(),
    );
    let req = TestRequest::default().to_http_request();
    assert_eq!(provider.extract_from_request(&req).unwrap(), None);
}

#[test]
fn pre_shared_key_body_location_always_returns_none() {
    let provider =
        PreSharedKeyAuthProvider::new("unused".to_string(), KeyLocation::Body, "psk".to_string());
    let req = TestRequest::default()
        .insert_header(("psk", "the-value"))
        .to_http_request();
    assert_eq!(provider.extract_from_request(&req).unwrap(), None);
}

#[test]
fn entity_jwt_extracts_bearer_token() {
    let provider = EntityJwtAuthProvider::new(None);
    let req = TestRequest::default()
        .insert_header(("Authorization", "Bearer abc.def.ghi"))
        .to_http_request();
    assert_eq!(
        provider.extract_from_request(&req).unwrap(),
        Some("abc.def.ghi".to_string())
    );
}

#[test]
fn entity_jwt_missing_authorization_header_returns_none() {
    let provider = EntityJwtAuthProvider::new(None);
    let req = TestRequest::default().to_http_request();
    assert_eq!(provider.extract_from_request(&req).unwrap(), None);
}

#[test]
fn entity_jwt_non_bearer_scheme_returns_none() {
    let provider = EntityJwtAuthProvider::new(None);
    let req = TestRequest::default()
        .insert_header(("Authorization", "Basic dXNlcjpwYXNz"))
        .to_http_request();
    assert_eq!(provider.extract_from_request(&req).unwrap(), None);
}
