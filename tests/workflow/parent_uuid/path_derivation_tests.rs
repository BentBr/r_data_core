#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for path derivation from `parent_uuid`.
//!
//! Tests the auto-generation of entity paths from parent relationships.

use super::helpers::{create_instance_definition, create_submission_definition};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{DynamicEntityService, EntityDefinitionService};
use r_data_core_test_support::{setup_test_db, unique_entity_type};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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
    // Note: `path` stores the parent path, not the full path.
    // Full path = path + '/' + entity_key = '/Clients' + '/' + 'my-customer' = '/Clients/my-customer'
    let mut parent_field_data: HashMap<String, Value> = HashMap::new();
    parent_field_data.insert("entity_key".to_string(), json!("my-customer"));
    parent_field_data.insert("path".to_string(), json!("/Clients"));
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
    // The `path` field stores the PARENT path (parent's full path: parent.path + "/" + parent.entity_key)
    // The full path of this entity would be: path + "/" + entity_key = "/Clients/my-customer/my-submission-key"
    let entity_path = entity
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_path,
        Some("/Clients/my-customer"),
        "path field should be the parent's full path (parent.path + '/' + parent.entity_key)"
    );

    // Verify the entity_key is correct
    let entity_key = entity
        .field_data
        .get("entity_key")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_key,
        Some("my-submission-key"),
        "entity_key should be preserved"
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
