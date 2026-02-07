#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests for workflow `parent_uuid` resolution
//!
//! Tests the `resolve_entity_path` transform with `parent_uuid` resolution
//! and entity creation with auto-generated paths from `parent_uuid`.

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::schema::Schema;
use r_data_core_core::field::options::FieldValidation;
use r_data_core_core::field::types::FieldType;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::FieldDefinition;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{DynamicEntityService, EntityDefinitionService};
use r_data_core_test_support::{setup_test_db, unique_entity_type};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Create an Instance entity definition with `key_id` field
fn create_instance_definition(entity_type: &str) -> EntityDefinition {
    let mut schema_properties = HashMap::new();
    schema_properties.insert(
        "entity_type".to_string(),
        serde_json::Value::String(entity_type.to_string()),
    );

    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: "Instance".to_string(),
        description: Some("Test instance entity".to_string()),
        group_name: None,
        allow_children: true,
        icon: None,
        fields: vec![FieldDefinition {
            name: "key_id".to_string(),
            display_name: "Key ID".to_string(),
            field_type: FieldType::String,
            description: None,
            required: false,
            indexed: true,
            filterable: true,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        schema: Schema::new(schema_properties),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: true,
        version: 1,
    }
}

/// Create a `StatisticSubmission` entity definition
fn create_submission_definition(entity_type: &str) -> EntityDefinition {
    let mut schema_properties = HashMap::new();
    schema_properties.insert(
        "entity_type".to_string(),
        serde_json::Value::String(entity_type.to_string()),
    );

    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: "Statistic Submission".to_string(),
        description: Some("Test submission entity".to_string()),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "submission_id".to_string(),
                display_name: "Submission ID".to_string(),
                field_type: FieldType::String,
                description: None,
                required: true,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "license_key_id".to_string(),
                display_name: "License Key ID".to_string(),
                field_type: FieldType::String,
                description: None,
                required: false,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: Schema::new(schema_properties),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: true,
        version: 1,
    }
}

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
    let mut field_data: HashMap<String, Value> = HashMap::new();
    field_data.insert("entity_key".to_string(), json!("unlicensed"));
    field_data.insert("path".to_string(), json!("/Clients/unlicensed"));
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
    let mut field_data: HashMap<String, Value> = HashMap::new();
    field_data.insert("entity_key".to_string(), json!("my-customer"));
    field_data.insert("path".to_string(), json!("/Clients/my-customer"));
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

#[tokio::test]
async fn test_derive_path_from_parent_uuid() {
    let pool = setup_test_db().await;
    let instance_type = unique_entity_type("Instance");
    let submission_type = unique_entity_type("Submission");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create Instance entity definition
    let instance_def = create_instance_definition(&instance_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    // Create `StatisticSubmission` entity definition
    let submission_def = create_submission_definition(&submission_type);
    ed_service
        .create_entity_definition(&submission_def)
        .await
        .expect("Failed to create StatisticSubmission definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create a parent Instance entity
    let mut parent_field_data: HashMap<String, Value> = HashMap::new();
    parent_field_data.insert("entity_key".to_string(), json!("my-customer"));
    parent_field_data.insert("path".to_string(), json!("/Clients/my-customer"));
    parent_field_data.insert("key_id".to_string(), json!("test-key"));
    parent_field_data.insert("published".to_string(), json!(true));
    parent_field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let parent_instance = DynamicEntity {
        entity_type: instance_type.clone(),
        field_data: parent_field_data,
        definition: Arc::new(instance_def.clone()),
    };
    let parent_uuid = de_service
        .create_entity(&parent_instance)
        .await
        .expect("Failed to create parent Instance");

    // Create a `StatisticSubmission` with `parent_uuid` but no path
    let ctx = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type.clone(),
        produced: json!({
            "parent_uuid": parent_uuid.to_string(),
            "entity_key": "my-submission-key",
            "submission_id": "sub-001",
            "license_key_id": "test-key",
            "published": true
        }),
        path: None, // No path - should be derived from `parent_uuid`
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };

    r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx)
        .await
        .expect("create_entity should succeed");

    // Verify the created entity has the correct path derived from parent
    let filter: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("entity_key".to_string(), json!("my-submission-key"));
        map
    };
    let entities = de_service
        .filter_entities(&submission_type, 1, 0, Some(filter), None, None, None)
        .await
        .expect("filter_entities should succeed");

    assert_eq!(entities.len(), 1, "Should find one entity");
    let entity = &entities[0];

    // Check the path was derived correctly
    let entity_path = entity
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_path,
        Some("/Clients/my-customer/my-submission-key"),
        "Path should be derived from parent's path + entity_key"
    );

    // Check `parent_uuid` was preserved
    let entity_parent_uuid = entity
        .field_data
        .get("parent_uuid")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_parent_uuid,
        Some(parent_uuid.to_string().as_str()),
        "parent_uuid should be preserved"
    );
}

#[tokio::test]
async fn test_entity_creation_error_when_no_path_and_no_parent_uuid() {
    let pool = setup_test_db().await;
    let submission_type = unique_entity_type("Submission");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create `StatisticSubmission` entity definition
    let submission_def = create_submission_definition(&submission_type);
    ed_service
        .create_entity_definition(&submission_def)
        .await
        .expect("Failed to create StatisticSubmission definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Try to create an entity without path or `parent_uuid`
    let ctx = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type,
        produced: json!({
            "entity_key": "orphan-submission",
            "submission_id": "sub-orphan",
            "published": true
            // No parent_uuid!
        }),
        path: None, // No path!
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };

    let result =
        r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx).await;

    // Should fail because neither path nor `parent_uuid` is provided
    assert!(result.is_err(), "Should fail when no path or parent_uuid");
    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("Either 'path' or 'parent_uuid' must be provided"),
        "Error should mention missing path/parent_uuid: {err}"
    );
}

#[tokio::test]
async fn test_entity_creation_with_explicit_path_ignores_parent_uuid_for_path() {
    let pool = setup_test_db().await;
    let instance_type = unique_entity_type("Instance");
    let submission_type = unique_entity_type("Submission");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create Instance entity definition
    let instance_def = create_instance_definition(&instance_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    // Create `StatisticSubmission` entity definition
    let submission_def = create_submission_definition(&submission_type);
    ed_service
        .create_entity_definition(&submission_def)
        .await
        .expect("Failed to create StatisticSubmission definition");

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create a parent Instance entity
    let mut parent_field_data: HashMap<String, Value> = HashMap::new();
    parent_field_data.insert("entity_key".to_string(), json!("parent-instance"));
    parent_field_data.insert("path".to_string(), json!("/Clients/parent-instance"));
    parent_field_data.insert("key_id".to_string(), json!("parent-key"));
    parent_field_data.insert("published".to_string(), json!(true));
    parent_field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let parent_instance = DynamicEntity {
        entity_type: instance_type.clone(),
        field_data: parent_field_data,
        definition: Arc::new(instance_def.clone()),
    };
    let parent_uuid = de_service
        .create_entity(&parent_instance)
        .await
        .expect("Failed to create parent Instance");

    // Create with BOTH explicit path AND `parent_uuid` - path should take precedence
    let ctx = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type.clone(),
        produced: json!({
            "parent_uuid": parent_uuid.to_string(),
            "entity_key": "explicit-path-submission",
            "submission_id": "sub-explicit",
            "published": true
        }),
        path: Some("/Custom/explicit-path".to_string()), // Explicit path provided
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };

    r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx)
        .await
        .expect("create_entity should succeed");

    // Verify the entity uses the explicit path, not derived from parent
    let filter: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("entity_key".to_string(), json!("explicit-path-submission"));
        map
    };
    let entities = de_service
        .filter_entities(&submission_type, 1, 0, Some(filter), None, None, None)
        .await
        .expect("filter_entities should succeed");

    assert_eq!(entities.len(), 1, "Should find one entity");
    let entity = &entities[0];

    // Check the explicit path was used
    let entity_path = entity
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_path,
        Some("/Custom/explicit-path"),
        "Path should be the explicitly provided path"
    );
}
