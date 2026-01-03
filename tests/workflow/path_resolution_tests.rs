#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_workflow::dsl::path_resolution::{
    apply_filters_transforms, apply_value_transform, build_path_from_fields, parse_entity_path,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_apply_value_transform_lowercase() {
    let value = json!("Hello World");
    let result = apply_value_transform(&value, "lowercase");
    assert_eq!(result, json!("hello world"));
}

#[test]
fn test_apply_value_transform_uppercase() {
    let value = json!("Hello World");
    let result = apply_value_transform(&value, "uppercase");
    assert_eq!(result, json!("HELLO WORLD"));
}

#[test]
fn test_apply_value_transform_trim() {
    let value = json!("  Hello World  ");
    let result = apply_value_transform(&value, "trim");
    assert_eq!(result, json!("Hello World"));
}

#[test]
fn test_apply_value_transform_normalize() {
    let value = json!("Hello@World#123");
    let result = apply_value_transform(&value, "normalize");
    assert_eq!(result, json!("Hello World 123"));
}

#[test]
fn test_apply_value_transform_slug() {
    let value = json!("Hello World Test");
    let result = apply_value_transform(&value, "slug");
    assert_eq!(result, json!("hello-world-test"));
}

#[test]
fn test_apply_value_transform_hash() {
    let value = json!("test");
    let result1 = apply_value_transform(&value, "hash");
    let result2 = apply_value_transform(&value, "hash");
    // Hash should be deterministic
    assert_eq!(result1, result2);
    // Hash should be different from input
    assert_ne!(result1, value);
}

#[test]
fn test_apply_value_transform_number() {
    let value = json!(123);
    let result = apply_value_transform(&value, "lowercase");
    assert_eq!(result, json!("123"));
}

#[test]
fn test_apply_value_transform_unknown() {
    let value = json!("test");
    let result = apply_value_transform(&value, "unknown_transform");
    // Should return original value
    assert_eq!(result, json!("test"));
}

#[test]
fn test_apply_filters_transforms() {
    let mut filters = HashMap::new();
    filters.insert("field1".to_string(), json!("  TEST  "));
    filters.insert("field2".to_string(), json!("Hello World"));

    let mut transforms = HashMap::new();
    transforms.insert("field1".to_string(), "trim".to_string());
    transforms.insert("field2".to_string(), "lowercase".to_string());

    let result = apply_filters_transforms::<std::collections::hash_map::RandomState>(
        &filters,
        Some(&transforms),
    );

    assert_eq!(result.get("field1"), Some(&json!("TEST")));
    assert_eq!(result.get("field2"), Some(&json!("hello world")));
}

#[test]
fn test_apply_filters_transforms_no_transforms() {
    let mut filters = HashMap::new();
    filters.insert("field1".to_string(), json!("test"));
    filters.insert("field2".to_string(), json!(123));

    let result = apply_filters_transforms(&filters, None);

    assert_eq!(result.get("field1"), Some(&json!("test")));
    assert_eq!(result.get("field2"), Some(&json!(123)));
}

#[test]
fn test_build_path_from_fields_simple() {
    let input = json!({
        "license_key_id": "ABC-123",
        "instance_name": "Production"
    });
    let template = "/statistics_instance/{license_key_id}";
    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template, &input, None, None,
    )
    .unwrap();
    assert_eq!(result, "/statistics_instance/ABC-123");
}

#[test]
fn test_build_path_from_fields_multiple() {
    let input = json!({
        "license_key_id": "ABC-123",
        "instance_name": "Production"
    });
    let template = "/statistics_instance/{license_key_id}/{instance_name}";
    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template, &input, None, None,
    )
    .unwrap();
    assert_eq!(result, "/statistics_instance/ABC-123/Production");
}

#[test]
fn test_build_path_from_fields_with_transforms() {
    let input = json!({
        "license_key_id": "ABC-123",
        "instance_name": "Production Instance"
    });
    let template = "/statistics_instance/{license_key_id}/{instance_name}";
    let mut transforms = HashMap::new();
    transforms.insert("license_key_id".to_string(), "lowercase".to_string());
    transforms.insert("instance_name".to_string(), "slug".to_string());

    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template,
        &input,
        None,
        Some(&transforms),
    )
    .unwrap();
    assert_eq!(result, "/statistics_instance/abc-123/production-instance");
}

#[test]
fn test_build_path_from_fields_custom_separator() {
    let input = json!({
        "part1": "a",
        "part2": "b"
    });
    let template = "{part1}-{part2}";
    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template,
        &input,
        Some("-"),
        None,
    )
    .unwrap();
    assert_eq!(result, "/a-b");
}

#[test]
fn test_build_path_from_fields_missing_field() {
    let input = json!({
        "license_key_id": "ABC-123"
    });
    let template = "/statistics_instance/{license_key_id}/{missing_field}";
    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template, &input, None, None,
    );
    assert!(result.is_err());
}

#[test]
fn test_build_path_from_fields_null_field() {
    let input = json!({
        "license_key_id": null
    });
    let template = "/statistics_instance/{license_key_id}";
    let result = build_path_from_fields::<std::collections::hash_map::RandomState>(
        template, &input, None, None,
    );
    assert!(result.is_err());
}

#[test]
fn test_parse_entity_path_simple() {
    let (path, key, parent) = parse_entity_path("/statistics_instance/abc-123");
    assert_eq!(path, "/statistics_instance/abc-123");
    assert_eq!(key, "abc-123");
    assert_eq!(parent, Some("/statistics_instance".to_string()));
}

#[test]
fn test_parse_entity_path_nested() {
    let (path, key, parent) = parse_entity_path("/statistics_instance/abc-123/submission-1");
    assert_eq!(path, "/statistics_instance/abc-123/submission-1");
    assert_eq!(key, "submission-1");
    assert_eq!(parent, Some("/statistics_instance/abc-123".to_string()));
}

#[test]
fn test_parse_entity_path_root() {
    let (path, key, parent) = parse_entity_path("/");
    assert_eq!(path, "/");
    assert_eq!(key, "");
    assert_eq!(parent, None);
}

#[test]
fn test_parse_entity_path_no_leading_slash() {
    let (path, key, parent) = parse_entity_path("statistics_instance/abc-123");
    assert_eq!(path, "/statistics_instance/abc-123");
    assert_eq!(key, "abc-123");
    assert_eq!(parent, Some("/statistics_instance".to_string()));
}

#[test]
fn test_parse_entity_path_single_segment() {
    let (path, key, parent) = parse_entity_path("/abc-123");
    assert_eq!(path, "/abc-123");
    assert_eq!(key, "abc-123");
    assert_eq!(parent, None);
}
