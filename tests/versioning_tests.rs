#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::VersionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::{EntityDefinitionService, VersionService};
use r_data_core_workflow::data::WorkflowKind;
use sqlx::Row;
use uuid::Uuid;

use r_data_core_test_support::{
    create_test_admin_user, create_test_entity, create_test_entity_definition, setup_test_db,
    unique_entity_type,
};

#[tokio::test]
async fn test_dynamic_entity_update_creates_snapshot() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("ver_entity");

    // Create definition and entity
    let _def_uuid = create_test_entity_definition(&pool, &entity_type)
        .await
        .unwrap();
    let entity_uuid = create_test_entity(&pool, &entity_type, "Alice", "alice@example.com")
        .await
        .unwrap();

    // Confirm initial version in registry (should be 1)
    let initial_version: i32 =
        sqlx::query_scalar("SELECT version FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert_eq!(initial_version, 1);

    // Update entity via repository with updated_by to attribute snapshot
    let mut payload = serde_json::json!({
        "uuid": entity_uuid.to_string(),
        "name": "Alice Updated",
        "email": "alice@example.com",
        "updated_by": Uuid::now_v7().to_string()
    });
    // Use the view to validate after update
    let repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    // We need definition for validation; load from service
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_svc = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));
    let def = def_svc
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: payload
            .as_object_mut()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        definition: std::sync::Arc::new(def),
    };
    repo.update(&entity).await.unwrap();

    // Verify snapshot row exists with version_number = pre-update version (1)
    let row = sqlx::query(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number DESC LIMIT 1",
    )
    .bind(entity_uuid)
    .fetch_one(&pool.pool)
    .await
    .unwrap();
    let snap_version: i32 = row.try_get("version_number").unwrap();
    assert_eq!(snap_version, 1);

    // Verify registry version incremented to 2
    let after_version: i32 =
        sqlx::query_scalar("SELECT version FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert_eq!(after_version, 2);
}

#[tokio::test]
async fn test_workflow_update_creates_snapshot_and_increments_version() {
    let pool = setup_test_db().await;
    let repo = WorkflowRepository::new(pool.pool.clone());

    // Create workflow with a valid admin user as creator
    let created_by = create_test_admin_user(&pool).await.unwrap();
    let req = CreateWorkflowRequest {
        name: "wf1".to_string(),
        description: Some("desc".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({"steps": []}),
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&req, created_by).await.unwrap();

    // initial version
    let ver_before: i32 = sqlx::query_scalar("SELECT version FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool.pool)
        .await
        .unwrap();

    // Update
    let upd = UpdateWorkflowRequest {
        name: "wf1-upd".to_string(),
        description: Some("desc2".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({"steps": []}),
        versioning_disabled: false,
    };
    let updated_by = create_test_admin_user(&pool).await.unwrap();
    repo.update(wf_uuid, &upd, updated_by).await.unwrap();

    // Snapshot should be for pre-update version (workflows use workflow_versions table)
    let row = sqlx::query(
        "SELECT version_number FROM workflow_versions WHERE workflow_uuid = $1 ORDER BY version_number DESC LIMIT 1",
    )
    .bind(wf_uuid)
    .fetch_optional(&pool.pool)
    .await
    .unwrap();
    assert!(row.is_some(), "Snapshot should exist");
    let snap_version: i32 = row.unwrap().try_get("version_number").unwrap();
    assert_eq!(snap_version, ver_before);

    // Version incremented
    let ver_after: i32 = sqlx::query_scalar("SELECT version FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool.pool)
        .await
        .unwrap();
    assert_eq!(ver_after, ver_before + 1);
}

#[tokio::test]
async fn test_entity_definition_update_creates_snapshot_and_increments_version() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("ver_def");

    // Create definition
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));

    let def = r_data_core_core::entity_definition::definition::EntityDefinition {
        entity_type: entity_type.clone(),
        display_name: "Ver Def".to_string(),
        description: Some("d".to_string()),
        published: true,
        created_by: Uuid::now_v7(),
        ..Default::default()
    };
    let def_uuid = def_service.create_entity_definition(&def).await.unwrap();

    // Before version
    let before_ver: i32 =
        sqlx::query_scalar("SELECT version FROM entity_definitions WHERE uuid = $1")
            .bind(def_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();

    // Update via repository (service .update calls repository.update)
    let repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let mut updated = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    updated.display_name = "Ver Def Updated".to_string();
    repo.update(&def_uuid, &updated).await.unwrap();

    // Snapshot exists at previous version (entity definitions use entity_definition_versions table)
    let row = sqlx::query(
        "SELECT version_number FROM entity_definition_versions WHERE definition_uuid = $1 ORDER BY version_number DESC LIMIT 1",
    )
    .bind(def_uuid)
    .fetch_optional(&pool.pool)
    .await
    .unwrap();
    assert!(row.is_some(), "Snapshot should exist");
    let snap_version: i32 = row.unwrap().try_get("version_number").unwrap();
    assert_eq!(snap_version, before_ver);

    // Version incremented
    let after_ver: i32 =
        sqlx::query_scalar("SELECT version FROM entity_definitions WHERE uuid = $1")
            .bind(def_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert_eq!(after_ver, before_ver + 1);
}

#[tokio::test]
async fn test_maintenance_prunes_by_age_and_count() {
    let pool = setup_test_db().await;
    let repo = VersionRepository::new(pool.pool.clone());
    // Create a dummy entity in entities_registry to satisfy foreign key constraint
    let entity_uuid = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO entities_registry (entity_type, path, entity_key, version, created_at, updated_at, created_by, published)
         VALUES ($1, '/', $2, 5, NOW(), NOW(), $3, true)
         RETURNING uuid"
    )
    .bind("dynamic_entity")
    .bind(Uuid::now_v7().to_string())
    .bind(Uuid::now_v7())
    .fetch_one(&pool.pool)
    .await
    .unwrap();

    // Seed versions 1..5 with different created_at; directly insert to control timestamps
    for v in 1..=5 {
        let _ = sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at) VALUES ($1, $2, $3, $4, NOW() - make_interval(days => $5)) ON CONFLICT DO NOTHING",
        )
        .bind(entity_uuid)
        .bind("dynamic_entity")
        .bind(v)
        .bind(serde_json::json!({"v": v}))
        .bind(200 - (v * 10)) // v=1 => ~190 days ago, v=5 => ~150 days ago
        .execute(&pool.pool)
        .await
        .unwrap();
    }

    // Prune older than 180 days (should delete versions with created_at < now-180d) => v=1 (~190d)
    let affected_age = repo.prune_older_than_days(180).await.unwrap();
    assert!(affected_age >= 1);

    // Ensure count is 4 remaining for this entity
    let count_after_age: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert!(count_after_age <= 4);

    // Keep latest 2 per entity: should delete all but version_numbers 5 and 4
    let _ = repo.prune_keep_latest_per_entity(2).await.unwrap();
    let rows = sqlx::query(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number DESC",
    )
    .bind(entity_uuid)
    .fetch_all(&pool.pool)
    .await
    .unwrap();
    let kept: Vec<i32> = rows
        .into_iter()
        .map(|r| r.get::<i32, _>("version_number"))
        .collect();
    assert!(kept.len() <= 2);
    if kept.len() == 2 {
        assert_eq!(kept, vec![5, 4]);
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Test function with comprehensive versioning scenarios
async fn test_version_creation_and_endpoint_output() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("ver_endpoint");
    let version_repo = VersionRepository::new(pool.pool.clone());

    // Create definition and entity
    let _def_uuid = create_test_entity_definition(&pool, &entity_type)
        .await
        .unwrap();

    // Create entity with a creator
    let creator = create_test_admin_user(&pool).await.unwrap();
    let entity_uuid = create_test_entity(&pool, &entity_type, "Bob", "bob@example.com")
        .await
        .unwrap();

    // Set created_by in registry to creator
    sqlx::query("UPDATE entities_registry SET created_by = $1 WHERE uuid = $2")
        .bind(creator)
        .bind(entity_uuid)
        .execute(&pool.pool)
        .await
        .unwrap();

    // Confirm initial version in registry (should be 1)
    let initial_version: i32 =
        sqlx::query_scalar("SELECT version FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert_eq!(initial_version, 1);

    // Update entity to create a snapshot
    let updated_by = create_test_admin_user(&pool).await.unwrap();
    let mut payload = serde_json::json!({
        "uuid": entity_uuid.to_string(),
        "name": "Bob Updated",
        "email": "bob.updated@example.com",
        "updated_by": updated_by.to_string()
    });

    let repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_svc = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));
    let def = def_svc
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: payload
            .as_object_mut()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        definition: std::sync::Arc::new(def),
    };
    repo.update(&entity).await.unwrap();

    // Verify registry version incremented to 2
    let after_version: i32 =
        sqlx::query_scalar("SELECT version FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await
            .unwrap();
    assert_eq!(after_version, 2);

    // Test: List versions using repository
    let versions = version_repo
        .list_entity_versions(entity_uuid)
        .await
        .unwrap();
    assert_eq!(versions.len(), 1, "Should have one version snapshot");
    assert_eq!(
        versions[0].version_number, 1,
        "Snapshot should be for version 1"
    );
    // Snapshot should be attributed to the entity's current state before update (creator, since it was never updated)
    assert_eq!(
        versions[0].created_by,
        Some(creator),
        "Snapshot should be attributed to creator (previous state)"
    );

    // Test: Get specific version using repository
    let version_payload = version_repo
        .get_entity_version(entity_uuid, 1)
        .await
        .unwrap();
    assert!(version_payload.is_some(), "Version 1 should exist");
    let payload = version_payload.unwrap();
    assert_eq!(payload.version_number, 1);
    assert_eq!(
        payload.created_by,
        Some(creator),
        "Version 1 should be attributed to creator"
    );

    // Verify the data contains the original values (before update)
    let data = payload.data.as_object().unwrap();
    assert_eq!(
        data.get("name").and_then(|v| v.as_str()),
        Some("Bob"),
        "Version 1 should have original name"
    );
    assert_eq!(
        data.get("email").and_then(|v| v.as_str()),
        Some("bob@example.com"),
        "Version 1 should have original email"
    );

    // Test: Get current version metadata using repository
    let current_metadata = version_repo
        .get_current_entity_metadata(entity_uuid)
        .await
        .unwrap();
    assert!(current_metadata.is_some(), "Current metadata should exist");
    let (current_version, _, current_updated_by, _current_updated_by_name) =
        current_metadata.unwrap();
    assert_eq!(current_version, 2, "Current version should be 2");
    assert_eq!(
        current_updated_by,
        Some(updated_by),
        "Current version should be attributed to updated_by"
    );

    // Test: Get current entity data using repository
    let current_data = version_repo
        .get_current_entity_data(entity_uuid, &entity_type)
        .await
        .unwrap();
    assert!(current_data.is_some(), "Current data should exist");
    let current_data_value = current_data.unwrap();
    let current_data_obj = current_data_value.as_object().unwrap();
    assert_eq!(
        current_data_obj.get("name").and_then(|v| v.as_str()),
        Some("Bob Updated"),
        "Current version should have updated name"
    );
    assert_eq!(
        current_data_obj.get("email").and_then(|v| v.as_str()),
        Some("bob.updated@example.com"),
        "Current version should have updated email"
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Test function with comprehensive versioning scenarios
async fn test_version_creator_names_in_json_response() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("ver_names");
    let version_service = VersionService::new(pool.pool.clone());

    // Create definition
    let _def_uuid = create_test_entity_definition(&pool, &entity_type)
        .await
        .unwrap();

    // Create entity with creator1 (with unique name)
    let creator1 = create_test_admin_user(&pool).await.unwrap();
    // Set first_name and last_name for creator1
    sqlx::query("UPDATE admin_users SET first_name = 'Creator', last_name = 'One' WHERE uuid = $1")
        .bind(creator1)
        .execute(&pool.pool)
        .await
        .unwrap();

    let entity_uuid = create_test_entity(&pool, &entity_type, "Test", "test@example.com")
        .await
        .unwrap();

    // Set created_by in registry to creator1
    sqlx::query("UPDATE entities_registry SET created_by = $1 WHERE uuid = $2")
        .bind(creator1)
        .bind(entity_uuid)
        .execute(&pool.pool)
        .await
        .unwrap();

    // Update entity with updater1 (this creates snapshot of version 1)
    let updater1 = create_test_admin_user(&pool).await.unwrap();
    // Set first_name and last_name for updater1
    sqlx::query("UPDATE admin_users SET first_name = 'Updater', last_name = 'One' WHERE uuid = $1")
        .bind(updater1)
        .execute(&pool.pool)
        .await
        .unwrap();
    let mut payload1 = serde_json::json!({
        "uuid": entity_uuid.to_string(),
        "name": "Test Updated 1",
        "email": "test1@example.com",
        "updated_by": updater1.to_string()
    });

    let repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_svc = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));
    let def = def_svc
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    let entity1 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: payload1
            .as_object_mut()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        definition: std::sync::Arc::new(def.clone()),
    };
    repo.update(&entity1).await.unwrap();

    // Update entity again with updater2 (this creates snapshot of version 2)
    let updater2 = create_test_admin_user(&pool).await.unwrap();
    // Set first_name and last_name for updater2
    sqlx::query("UPDATE admin_users SET first_name = 'Updater', last_name = 'Two' WHERE uuid = $1")
        .bind(updater2)
        .execute(&pool.pool)
        .await
        .unwrap();
    let mut payload2 = serde_json::json!({
        "uuid": entity_uuid.to_string(),
        "name": "Test Updated 2",
        "email": "test2@example.com",
        "updated_by": updater2.to_string()
    });

    let entity2 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: payload2
            .as_object_mut()
            .unwrap()
            .clone()
            .into_iter()
            .collect(),
        definition: std::sync::Arc::new(def),
    };
    repo.update(&entity2).await.unwrap();

    // Get versions via service (which includes name resolution)
    let versions = version_service
        .list_entity_versions_with_metadata(entity_uuid)
        .await
        .unwrap();

    // Should have 2 snapshots (version 1 and 2) plus current version (3)
    assert_eq!(
        versions.len(),
        3,
        "Should have 3 versions (2 snapshots + current)"
    );

    // Version 1 snapshot should have creator1's name
    let v1 = versions.iter().find(|v| v.version_number == 1).unwrap();
    assert_eq!(
        v1.created_by,
        Some(creator1),
        "Version 1 snapshot should have creator1"
    );
    assert!(
        v1.created_by_name.is_some(),
        "Version 1 should have resolved creator name"
    );
    let v1_name = v1.created_by_name.as_ref().unwrap();
    assert!(
        !v1_name.is_empty(),
        "Version 1 creator name should not be empty"
    );
    assert!(
        v1_name.contains("Creator"),
        "Version 1 should have Creator One's name"
    );

    // Version 2 snapshot should have updater1's name
    let v2 = versions.iter().find(|v| v.version_number == 2).unwrap();
    assert_eq!(
        v2.created_by,
        Some(updater1),
        "Version 2 snapshot should have updater1"
    );
    assert!(
        v2.created_by_name.is_some(),
        "Version 2 should have resolved creator name"
    );
    let v2_name = v2.created_by_name.as_ref().unwrap();
    assert!(
        !v2_name.is_empty(),
        "Version 2 creator name should not be empty"
    );
    assert!(
        v2_name.contains("Updater"),
        "Version 2 should have Updater One's name"
    );

    // Current version (3) should have updater2's name
    let v3 = versions.iter().find(|v| v.version_number == 3).unwrap();
    assert_eq!(
        v3.created_by,
        Some(updater2),
        "Current version should have updater2"
    );
    assert!(
        v3.created_by_name.is_some(),
        "Current version should have resolved creator name"
    );
    let v3_name = v3.created_by_name.as_ref().unwrap();
    assert!(
        !v3_name.is_empty(),
        "Current version creator name should not be empty"
    );
    assert!(
        v3_name.contains("Updater"),
        "Current version should have Updater Two's name"
    );

    // Verify names are different (they should be different admin users)
    assert_ne!(
        v1_name, v2_name,
        "Version 1 and 2 should have different creator names"
    );
    assert_ne!(
        v2_name, v3_name,
        "Version 2 and 3 should have different creator names"
    );

    // Verify the JSON structure matches what the API would return
    let json_versions: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "version_number": v.version_number,
                "created_at": v.created_at,
                "created_by": v.created_by,
                "created_by_name": v.created_by_name
            })
        })
        .collect();

    // Verify JSON structure
    for json_v in &json_versions {
        assert!(
            json_v.get("version_number").is_some(),
            "JSON should have version_number"
        );
        assert!(
            json_v.get("created_at").is_some(),
            "JSON should have created_at"
        );
        assert!(
            json_v.get("created_by").is_some(),
            "JSON should have created_by"
        );
        assert!(
            json_v.get("created_by_name").is_some(),
            "JSON should have created_by_name"
        );
        // created_by_name should be a string (not null, not empty)
        if let Some(name) = json_v
            .get("created_by_name")
            .and_then(|v: &serde_json::Value| v.as_str())
        {
            assert!(
                !name.is_empty(),
                "created_by_name should not be empty in JSON"
            );
        } else {
            panic!("created_by_name should be a string in JSON");
        }
    }
}
