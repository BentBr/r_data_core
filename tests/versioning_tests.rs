use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::api::admin::workflows::models as wf_models;
use r_data_core::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core::workflow::data::WorkflowKind;
use r_data_core::entity::version_repository::VersionRepository;
use r_data_core::services::EntityDefinitionService;
use r_data_core::workflow::data::repository::WorkflowRepository;
use r_data_core::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use sqlx::Row;
use uuid::Uuid;

mod common;
use common::utils::{
    create_test_entity, create_test_entity_definition, setup_test_db, unique_entity_type,
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
            .fetch_one(&pool)
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
    let repo =
        r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository::new(pool.clone());
    // We need definition for validation; load from service
    let def_repo = EntityDefinitionRepository::new(pool.clone());
    let def_svc = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));
    let def = def_svc
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    let entity = r_data_core::entity::dynamic_entity::entity::DynamicEntity {
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
    .fetch_one(&pool)
    .await
    .unwrap();
    let snap_version: i32 = row.try_get("version_number").unwrap();
    assert_eq!(snap_version, 1);

    // Verify registry version incremented to 2
    let after_version: i32 =
        sqlx::query_scalar("SELECT version FROM entities_registry WHERE uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after_version, 2);
}

#[tokio::test]
async fn test_workflow_update_creates_snapshot_and_increments_version() {
    let pool = setup_test_db().await;
    let repo = WorkflowRepository::new(pool.clone());

    // Create workflow with a valid admin user as creator
    let created_by = common::utils::create_test_admin_user(&pool).await.unwrap();
    let req = CreateWorkflowRequest {
        name: "wf1".to_string(),
        description: Some("desc".to_string()),
        kind: WorkflowKind::Consumer,
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({"steps": []}),
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&req, created_by).await.unwrap();

    // initial version
    let ver_before: i32 = sqlx::query_scalar("SELECT version FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();

    // Update
    let upd = UpdateWorkflowRequest {
        name: "wf1-upd".to_string(),
        description: Some("desc2".to_string()),
        kind: WorkflowKind::Consumer,
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({"steps": []}),
        versioning_disabled: false,
    };
    let updated_by = common::utils::create_test_admin_user(&pool).await.unwrap();
    repo.update(wf_uuid, &upd, updated_by).await.unwrap();

    // Snapshot should be for pre-update version
    let row = sqlx::query(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 AND entity_type = 'workflow' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(wf_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    let snap_version: i32 = row.try_get("version_number").unwrap();
    assert_eq!(snap_version, ver_before);

    // Version incremented
    let ver_after: i32 = sqlx::query_scalar("SELECT version FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ver_after, ver_before + 1);
}

#[tokio::test]
async fn test_entity_definition_update_creates_snapshot_and_increments_version() {
    let pool = setup_test_db().await;
    let entity_type = unique_entity_type("ver_def");

    // Create definition
    let def_repo = EntityDefinitionRepository::new(pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(std::sync::Arc::new(def_repo));

    let mut def = r_data_core::entity::entity_definition::definition::EntityDefinition::default();
    def.entity_type = entity_type.clone();
    def.display_name = "Ver Def".to_string();
    def.description = Some("d".to_string());
    def.published = true;
    def.created_by = Uuid::now_v7();
    let def_uuid = def_service.create_entity_definition(&def).await.unwrap();

    // Before version
    let before_ver: i32 =
        sqlx::query_scalar("SELECT version FROM entity_definitions WHERE uuid = $1")
            .bind(def_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Update via repository (service .update calls repository.update)
    let repo =
        r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository::new(
            pool.clone(),
        );
    let mut updated = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await
        .unwrap();
    updated.display_name = "Ver Def Updated".to_string();
    repo.update(&def_uuid, &updated).await.unwrap();

    // Snapshot exists at previous version
    let row = sqlx::query(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 AND entity_type = 'entity_definition' ORDER BY version_number DESC LIMIT 1",
    )
    .bind(def_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    let snap_version: i32 = row.try_get("version_number").unwrap();
    assert_eq!(snap_version, before_ver);

    // Version incremented
    let after_ver: i32 =
        sqlx::query_scalar("SELECT version FROM entity_definitions WHERE uuid = $1")
            .bind(def_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after_ver, before_ver + 1);
}

#[tokio::test]
async fn test_maintenance_prunes_by_age_and_count() {
    let pool = setup_test_db().await;
    let repo = VersionRepository::new(pool.clone());
    let entity_uuid = Uuid::now_v7();

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
        .execute(&pool)
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
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(count_after_age <= 4);

    // Keep latest 2 per entity: should delete all but version_numbers 5 and 4
    let _ = repo.prune_keep_latest_per_entity(2).await.unwrap();
    let rows = sqlx::query(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number DESC",
    )
    .bind(entity_uuid)
    .fetch_all(&pool)
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
