#![allow(clippy::unwrap_used)]

use serde_json::json;
use uuid::Uuid;

use crate::domain::dynamic_entity::entity::{FromValue, ToValue};

use super::create_test_entity;

// --- String FromValue ---

#[test]
fn test_from_value_string_from_string() {
    let v = json!("hello");
    assert_eq!(String::from_value(&v).unwrap(), "hello");
}

#[test]
fn test_from_value_string_from_number() {
    let v = json!(42);
    assert_eq!(String::from_value(&v).unwrap(), "42");
}

#[test]
fn test_from_value_string_from_bool() {
    let v = json!(true);
    assert_eq!(String::from_value(&v).unwrap(), "true");
}

#[test]
fn test_from_value_string_from_null_gives_empty() {
    let v = serde_json::Value::Null;
    assert!(String::from_value(&v).unwrap().is_empty());
}

#[test]
fn test_from_value_string_from_array_fails() {
    let v = json!([1, 2]);
    assert!(String::from_value(&v).is_err());
}

#[test]
fn test_from_value_string_from_object_fails() {
    let v = json!({"a": 1});
    assert!(String::from_value(&v).is_err());
}

// --- i64 FromValue ---

#[test]
fn test_from_value_i64_from_number() {
    let v = json!(7i64);
    assert_eq!(i64::from_value(&v).unwrap(), 7);
}

#[test]
fn test_from_value_i64_from_string() {
    let v = json!("99");
    assert_eq!(i64::from_value(&v).unwrap(), 99);
}

#[test]
fn test_from_value_i64_from_bad_string_fails() {
    let v = json!("abc");
    assert!(i64::from_value(&v).is_err());
}

#[test]
fn test_from_value_i64_from_bool_fails() {
    let v = json!(true);
    assert!(i64::from_value(&v).is_err());
}

// --- bool FromValue ---

#[test]
fn test_from_value_bool_true() {
    assert!(bool::from_value(&json!(true)).unwrap());
}

#[test]
fn test_from_value_bool_false() {
    assert!(!bool::from_value(&json!(false)).unwrap());
}

#[test]
fn test_from_value_bool_from_nonzero_number() {
    assert!(bool::from_value(&json!(1)).unwrap());
}

#[test]
fn test_from_value_bool_from_zero() {
    assert!(!bool::from_value(&json!(0)).unwrap());
}

#[test]
fn test_from_value_bool_from_string_true() {
    assert!(bool::from_value(&json!("true")).unwrap());
}

#[test]
fn test_from_value_bool_from_string_one() {
    assert!(bool::from_value(&json!("1")).unwrap());
}

#[test]
fn test_from_value_bool_from_string_false() {
    assert!(!bool::from_value(&json!("false")).unwrap());
}

#[test]
fn test_from_value_bool_from_null_fails() {
    assert!(bool::from_value(&serde_json::Value::Null).is_err());
}

// --- Uuid FromValue ---

#[test]
fn test_from_value_uuid_from_valid_string() {
    let id = Uuid::nil();
    let v = json!(id.to_string());
    assert_eq!(Uuid::from_value(&v).unwrap(), id);
}

#[test]
fn test_from_value_uuid_from_bad_string_fails() {
    assert!(Uuid::from_value(&json!("not-a-uuid")).is_err());
}

#[test]
fn test_from_value_uuid_from_non_string_fails() {
    assert!(Uuid::from_value(&json!(42)).is_err());
}

// --- ToValue ---

#[test]
fn test_to_value_string() {
    let s = "hello".to_string();
    assert_eq!(s.to_value().unwrap(), json!("hello"));
}

#[test]
fn test_to_value_i64() {
    assert_eq!(42i64.to_value().unwrap(), json!(42));
}

#[test]
fn test_to_value_bool() {
    assert_eq!(true.to_value().unwrap(), json!(true));
}

#[test]
fn test_to_value_json_value_clones() {
    let v = json!({"x": 1});
    assert_eq!(v.to_value().unwrap(), v);
}

// --- DynamicEntity::get typed ---

#[test]
fn test_entity_get_string_field() {
    let mut entity = create_test_entity();
    entity.set("name", "hello".to_string()).unwrap();
    let v: String = entity.get("name").unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn test_entity_get_i64_field() {
    let mut entity = create_test_entity();
    entity.set("count", 42i64).unwrap();
    let v: i64 = entity.get("count").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn test_entity_get_bool_field() {
    let mut entity = create_test_entity();
    entity.set("active", true).unwrap();
    assert!(entity.get::<bool>("active").unwrap());
}

#[test]
fn test_entity_get_missing_field_is_error() {
    let entity = create_test_entity();
    let result: crate::error::Result<String> = entity.get("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonexistent"));
}
