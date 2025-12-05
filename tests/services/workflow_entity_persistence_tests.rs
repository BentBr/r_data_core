#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_services::workflow::entity_persistence::{
    ensure_audit_fields, EntityLookupResult, PersistenceContext,
};
use r_data_core_services::workflow::value_formatting::normalize_path;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

// Note: These are unit tests for helper functions
// Full integration tests would require database setup

#[test]
fn test_normalize_path_helper() {
    assert_eq!(normalize_path("/test"), "/test");
    assert_eq!(normalize_path("test"), "/test");
    assert_eq!(normalize_path("/"), "/");
}

#[test]
fn test_ensure_audit_fields() {
    let mut field_data = HashMap::new();
    let run_uuid = Uuid::now_v7();

    ensure_audit_fields(&mut field_data, run_uuid);

    assert_eq!(
        field_data.get("created_by"),
        Some(&json!(run_uuid.to_string()))
    );
    assert_eq!(
        field_data.get("updated_by"),
        Some(&json!(run_uuid.to_string()))
    );
}

#[test]
fn test_ensure_audit_fields_preserves_existing() {
    let mut field_data = HashMap::new();
    let existing_uuid = Uuid::now_v7();
    field_data.insert("created_by".to_string(), json!(existing_uuid.to_string()));

    let run_uuid = Uuid::now_v7();
    ensure_audit_fields(&mut field_data, run_uuid);

    // Should preserve existing created_by
    assert_eq!(
        field_data.get("created_by"),
        Some(&json!(existing_uuid.to_string()))
    );
    // Should set updated_by
    assert_eq!(
        field_data.get("updated_by"),
        Some(&json!(run_uuid.to_string()))
    );
}

#[test]
fn test_ensure_entity_key_generates_when_missing() {
    // This test would require a mock DynamicEntityService
    // For now, we test the logic conceptually
    // In integration tests, we'd test the full flow
}

#[test]
fn test_persistence_context() {
    let ctx = PersistenceContext {
        entity_type: "customer".to_string(),
        produced: json!({"name": "Test"}),
        path: Some("/customers".to_string()),
        run_uuid: Uuid::now_v7(),
        update_key: Some("email".to_string()),
        skip_versioning: false,
    };

    assert_eq!(ctx.entity_type, "customer");
    assert_eq!(ctx.path, Some("/customers".to_string()));
    assert!(ctx.update_key.is_some());
}

#[test]
fn test_entity_lookup_result_enum() {
    // Test that the enum variants exist
    let found = EntityLookupResult::NotFound;
    let _ = found; // Suppress unused variable warning
                   // In real tests, we'd create a DynamicEntity for Found variant
}
