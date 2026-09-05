use super::safe_field;
use crate::dsl::transform::validate_transform;
use crate::dsl::transform::{AuthenticateTransform, Transform};
use std::collections::HashMap;

fn authenticate(entity_type: &str, extra_claims: HashMap<String, String>) -> Transform {
    Transform::Authenticate(AuthenticateTransform {
        entity_type: entity_type.to_string(),
        identifier_field: "email".to_string(),
        password_field: "password_hash".to_string(),
        input_identifier: "input_email".to_string(),
        input_password: "input_password".to_string(),
        target_token: "jwt".to_string(),
        extra_claims,
        token_expiry_seconds: None,
    })
}

#[test]
fn valid_authenticate_ok() {
    let t = authenticate("user", HashMap::new());
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn authenticate_empty_entity_type_fails() {
    let t = authenticate("", HashMap::new());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_unsafe_entity_type_fails() {
    let t = authenticate("bad type!", HashMap::new());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_unsafe_identifier_field_fails() {
    let mut t = authenticate("user", HashMap::new());
    if let Transform::Authenticate(ref mut a) = t {
        a.identifier_field = "bad field!".to_string();
    }
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_unsafe_target_token_fails() {
    let mut t = authenticate("user", HashMap::new());
    if let Transform::Authenticate(ref mut a) = t {
        a.target_token = "bad token!".to_string();
    }
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_empty_extra_claims_ok() {
    let t = authenticate("user", HashMap::new());
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn authenticate_extra_claims_empty_key_fails() {
    let claims = HashMap::from([(String::new(), "role".to_string())]);
    let t = authenticate("user", claims);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_extra_claims_unsafe_field_fails() {
    let claims = HashMap::from([("role".to_string(), "bad field!".to_string())]);
    let t = authenticate("user", claims);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn authenticate_extra_claims_valid_ok() {
    let claims = HashMap::from([("role".to_string(), "user_role".to_string())]);
    let t = authenticate("user", claims);
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn authenticate_with_token_expiry_override_ok() {
    let mut t = authenticate("user", HashMap::new());
    if let Transform::Authenticate(ref mut a) = t {
        a.token_expiry_seconds = Some(3600);
    }
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}
