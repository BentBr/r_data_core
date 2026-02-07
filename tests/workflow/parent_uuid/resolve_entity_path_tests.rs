#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `resolve_entity_path` function.
//!
//! Tests the ability to find entities by filters and resolve their paths,
//! including fallback behavior when entities are not found.

use super::helpers::create_instance_definition;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{DynamicEntityService, EntityDefinitionService};
use r_data_core_test_support::{setup_test_db, unique_entity_type};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_resolve_entity_path_with_fallback_returns_uuid() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("Instance");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create Instance entity definition
    let instance_def = create_instance_definition(&entity_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create the fallback Instance entity at /Clients/unlicensed
    // Note: The `path` field stores the PARENT path, not the full path.
    // Full path = path + '/' + entity_key = '/Clients' + '/' + 'unlicensed' = '/Clients/unlicensed'
    let mut field_data: HashMap<String, Value> = HashMap::new();
    field_data.insert("entity_key".to_string(), json!("unlicensed"));
    field_data.insert("path".to_string(), json!("/Clients"));
    field_data.insert("key_id".to_string(), json!("unlicensed"));
    field_data.insert("published".to_string(), json!(true));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let fallback_instance = DynamicEntity {
        entity_type: entity_type.clone(),
        field_data,
        definition: Arc::new(instance_def.clone()),
    };
    let fallback_uuid = de_service
        .create_entity(&fallback_instance)
        .await
        .expect("Failed to create fallback Instance");

    // Test `resolve_entity_path` with a non-existent `key_id` (should use fallback)
    let filters: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("key_id".to_string(), json!("non-existent-key"));
        map
    };

    let result = r_data_core_services::workflow::entity_persistence::resolve_entity_path(
        &entity_type,
        &filters,
        None,
        Some("/Clients/unlicensed"),
        &de_service,
    )
    .await
    .expect("resolve_entity_path should succeed");

    // Should return the fallback entity's path AND UUID
    assert!(result.is_some(), "Should return a result");
    let (path, entity_uuid) = result.unwrap();
    assert_eq!(path, "/Clients/unlicensed");
    assert!(
        entity_uuid.is_some(),
        "Should return the fallback entity's UUID"
    );
    assert_eq!(
        entity_uuid.unwrap(),
        fallback_uuid,
        "UUID should match fallback entity"
    );
}

#[tokio::test]
async fn test_resolve_entity_path_finds_existing_entity() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("Instance");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create Instance entity definition
    let instance_def = create_instance_definition(&entity_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create an Instance entity with a specific `key_id`
    // Note: The `path` field stores the PARENT path, not the full path.
    // Full path = path + '/' + entity_key = '/Clients' + '/' + 'my-customer' = '/Clients/my-customer'
    let mut field_data: HashMap<String, Value> = HashMap::new();
    field_data.insert("entity_key".to_string(), json!("my-customer"));
    field_data.insert("path".to_string(), json!("/Clients"));
    field_data.insert("key_id".to_string(), json!("test-license-key-123"));
    field_data.insert("published".to_string(), json!(true));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let instance = DynamicEntity {
        entity_type: entity_type.clone(),
        field_data,
        definition: Arc::new(instance_def.clone()),
    };
    let instance_uuid = de_service
        .create_entity(&instance)
        .await
        .expect("Failed to create Instance");

    // Test `resolve_entity_path` with the existing `key_id`
    let filters: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("key_id".to_string(), json!("test-license-key-123"));
        map
    };

    let result = r_data_core_services::workflow::entity_persistence::resolve_entity_path(
        &entity_type,
        &filters,
        None,
        Some("/Clients/unlicensed"),
        &de_service,
    )
    .await
    .expect("resolve_entity_path should succeed");

    // Should return the found entity's path and UUID
    assert!(result.is_some(), "Should return a result");
    let (path, entity_uuid) = result.unwrap();
    assert_eq!(path, "/Clients/my-customer");
    assert!(entity_uuid.is_some(), "Should return the entity's UUID");
    assert_eq!(
        entity_uuid.unwrap(),
        instance_uuid,
        "UUID should match found entity"
    );
}

#[tokio::test]
async fn test_resolve_entity_path_error_when_fallback_not_found() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("Instance");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create Instance entity definition
    let instance_def = create_instance_definition(&entity_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Don't create any Instance entities

    // Test `resolve_entity_path` with a non-existent `key_id` and non-existent fallback path
    let filters: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("key_id".to_string(), json!("non-existent-key"));
        map
    };

    let result = r_data_core_services::workflow::entity_persistence::resolve_entity_path(
        &entity_type,
        &filters,
        None,
        Some("/Clients/non-existent-fallback"),
        &de_service,
    )
    .await;

    // Should return an error because the fallback entity doesn't exist
    assert!(
        result.is_err(),
        "Should return an error when fallback entity not found"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Fallback entity not found"),
        "Error should mention fallback entity not found: {err}"
    );
}
