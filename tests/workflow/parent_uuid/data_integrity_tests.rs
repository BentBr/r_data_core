#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Data integrity tests for parent-path relationships.
//!
//! Tests ensure that the path field is ALWAYS consistent with the parent relationship.
//! Rule: path = `parent.path` + "/" + `parent.entity_key`

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
async fn test_parent_uuid_always_determines_path_for_data_integrity() {
    // This test verifies that when `parent_uuid` is set, the path is ALWAYS derived from the parent
    // to ensure data integrity. Explicit paths are ignored when parent_uuid is provided.
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
    // Full path = path + '/' + entity_key = '/Clients' + '/' + 'parent-instance'
    let mut parent_field_data: HashMap<String, Value> = HashMap::new();
    parent_field_data.insert("entity_key".to_string(), json!("parent-instance"));
    parent_field_data.insert("path".to_string(), json!("/Clients"));
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

    // Create with BOTH explicit path AND `parent_uuid`
    // IMPORTANT: When parent_uuid is set, path is ALWAYS derived from parent for data integrity
    // The explicit path "/Custom/explicit-path" should be IGNORED
    let ctx = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type.clone(),
        produced: json!({
            "parent_uuid": parent_uuid.to_string(),
            "entity_key": "child-submission",
            "submission_id": "sub-explicit",
            "published": true
        }),
        path: Some("/Custom/explicit-path".to_string()), // This should be IGNORED
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };

    r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx)
        .await
        .expect("create_entity should succeed");

    // Verify the entity uses path derived from parent, NOT the explicit path
    let filter: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("entity_key".to_string(), json!("child-submission"));
        map
    };
    let entities = de_service
        .filter_entities(&submission_type, 1, 0, Some(filter), None, None, None)
        .await
        .expect("filter_entities should succeed");

    assert_eq!(entities.len(), 1, "Should find one entity");
    let entity = &entities[0];

    // Check that path was derived from parent, not the explicit path
    // Parent has path="/Clients" and entity_key="parent-instance"
    // So parent's full path is "/Clients/parent-instance"
    // Child's path field should be the parent's full path
    let entity_path = entity
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str);
    assert_eq!(
        entity_path,
        Some("/Clients/parent-instance"),
        "path should be derived from parent (parent.path + '/' + parent.entity_key), not explicit path"
    );

    // Verify parent_uuid is set correctly
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

/// **DATA INTEGRITY TEST**: Verifies that path is ALWAYS consistent with parent relationship.
///
/// Rule: path = `parent.path` + "/" + `parent.entity_key`
///
/// This ensures:
/// 1. Entity tree hierarchy is always consistent
/// 2. Querying by path always finds entities in correct locations
/// 3. No orphaned entities with incorrect paths
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_path_parent_integrity_must_always_match() {
    let pool = setup_test_db().await;
    let instance_type = unique_entity_type("Instance");
    let submission_type = unique_entity_type("Submission");

    // Create services
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(Arc::new(entity_repo), Arc::new(ed_service.clone()));

    // Create entity definitions
    let instance_def = create_instance_definition(&instance_type);
    ed_service
        .create_entity_definition(&instance_def)
        .await
        .expect("Failed to create Instance definition");

    let submission_def = create_submission_definition(&submission_type);
    ed_service
        .create_entity_definition(&submission_def)
        .await
        .expect("Failed to create StatisticSubmission definition");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // === Test Case 1: Root-level parent ===
    // Parent at root: path="/", entity_key="root-parent"
    // Parent's full path: "/root-parent"
    let mut root_parent_data: HashMap<String, Value> = HashMap::new();
    root_parent_data.insert("entity_key".to_string(), json!("root-parent"));
    root_parent_data.insert("path".to_string(), json!("/"));
    root_parent_data.insert("key_id".to_string(), json!("root-key"));
    root_parent_data.insert("published".to_string(), json!(true));
    root_parent_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let root_parent = DynamicEntity {
        entity_type: instance_type.clone(),
        field_data: root_parent_data,
        definition: Arc::new(instance_def.clone()),
    };
    let root_parent_uuid = de_service
        .create_entity(&root_parent)
        .await
        .expect("Failed to create root parent");

    // Create child of root parent
    let ctx1 = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type.clone(),
        produced: json!({
            "parent_uuid": root_parent_uuid.to_string(),
            "entity_key": "child-of-root",
            "submission_id": "sub-root",
            "published": true
        }),
        path: None,
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };
    r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx1)
        .await
        .expect("create child of root should succeed");

    // Verify child's path = parent's full path = "/" + "root-parent" = "/root-parent"
    let filter1: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("entity_key".to_string(), json!("child-of-root"));
        map
    };
    let entities1 = de_service
        .filter_entities(&submission_type, 1, 0, Some(filter1), None, None, None)
        .await
        .expect("filter should succeed");
    assert_eq!(entities1.len(), 1);
    let child1 = &entities1[0];

    let child1_path = child1
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str)
        .expect("path should be set");
    let child1_parent_uuid = child1
        .field_data
        .get("parent_uuid")
        .and_then(serde_json::Value::as_str)
        .expect("parent_uuid should be set");

    assert_eq!(
        child1_path, "/root-parent",
        "Child's path must be parent's full path: parent.path + '/' + parent.entity_key"
    );
    assert_eq!(
        child1_parent_uuid,
        root_parent_uuid.to_string(),
        "parent_uuid must match"
    );

    // === Test Case 2: Nested parent (3 levels deep) ===
    // Grandparent: path="/Clients", entity_key="grandparent"
    // Grandparent's full path: "/Clients/grandparent"
    let mut grandparent_data: HashMap<String, Value> = HashMap::new();
    grandparent_data.insert("entity_key".to_string(), json!("grandparent"));
    grandparent_data.insert("path".to_string(), json!("/Clients"));
    grandparent_data.insert("key_id".to_string(), json!("gp-key"));
    grandparent_data.insert("published".to_string(), json!(true));
    grandparent_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let grandparent = DynamicEntity {
        entity_type: instance_type.clone(),
        field_data: grandparent_data,
        definition: Arc::new(instance_def.clone()),
    };
    let grandparent_uuid = de_service
        .create_entity(&grandparent)
        .await
        .expect("Failed to create grandparent");

    // Parent: path="/Clients/grandparent", entity_key="parent"
    // Parent's full path: "/Clients/grandparent/parent"
    let mut parent_data: HashMap<String, Value> = HashMap::new();
    parent_data.insert("entity_key".to_string(), json!("parent"));
    parent_data.insert("path".to_string(), json!("/Clients/grandparent"));
    parent_data.insert(
        "parent_uuid".to_string(),
        json!(grandparent_uuid.to_string()),
    );
    parent_data.insert("key_id".to_string(), json!("p-key"));
    parent_data.insert("published".to_string(), json!(true));
    parent_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let parent = DynamicEntity {
        entity_type: instance_type.clone(),
        field_data: parent_data,
        definition: Arc::new(instance_def.clone()),
    };
    let parent_uuid = de_service
        .create_entity(&parent)
        .await
        .expect("Failed to create parent");

    // Create child of nested parent
    let ctx2 = r_data_core_services::workflow::entity_persistence::PersistenceContext {
        entity_type: submission_type.clone(),
        produced: json!({
            "parent_uuid": parent_uuid.to_string(),
            "entity_key": "deep-child",
            "submission_id": "sub-deep",
            "published": true
        }),
        path: None,
        run_uuid: Uuid::now_v7(),
        update_key: None,
        skip_versioning: true,
    };
    r_data_core_services::workflow::entity_persistence::create_entity(&de_service, &ctx2)
        .await
        .expect("create deep child should succeed");

    // Verify child's path = parent's full path = "/Clients/grandparent" + "/" + "parent"
    let filter2: HashMap<String, Value> = {
        let mut map = HashMap::new();
        map.insert("entity_key".to_string(), json!("deep-child"));
        map
    };
    let entities2 = de_service
        .filter_entities(&submission_type, 1, 0, Some(filter2), None, None, None)
        .await
        .expect("filter should succeed");
    assert_eq!(entities2.len(), 1);
    let child2 = &entities2[0];

    let child2_path = child2
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str)
        .expect("path should be set");
    let child2_parent_uuid = child2
        .field_data
        .get("parent_uuid")
        .and_then(serde_json::Value::as_str)
        .expect("parent_uuid should be set");

    assert_eq!(
        child2_path, "/Clients/grandparent/parent",
        "Child's path must be parent's full path (3 levels deep): grandparent.path + '/' + grandparent.entity_key + '/' + parent.entity_key"
    );
    assert_eq!(
        child2_parent_uuid,
        parent_uuid.to_string(),
        "parent_uuid must match"
    );

    // === Integrity Check: Verify parent lookup matches stored parent_uuid ===
    // For child2, we can verify that looking up by parent_uuid gives us the entity
    // whose path + entity_key = child2.path
    let parent_entity = de_service
        .get_entity_by_uuid_any_type(parent_uuid)
        .await
        .expect("Parent lookup should succeed");

    let parent_path = parent_entity
        .field_data
        .get("path")
        .and_then(serde_json::Value::as_str)
        .expect("parent path");
    let parent_key = parent_entity
        .field_data
        .get("entity_key")
        .and_then(serde_json::Value::as_str)
        .expect("parent entity_key");

    let expected_child_path = format!("{parent_path}/{parent_key}");
    assert_eq!(
        child2_path, expected_child_path,
        "DATA INTEGRITY: child.path must equal parent.path + '/' + parent.entity_key"
    );
}
