use r_data_core_api::admin::workflows::models::CreateWorkflowRequest;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_services::adapters::DynamicEntityRepositoryAdapter;
use r_data_core_services::adapters::EntityDefinitionRepositoryAdapter;
use r_data_core_services::{DynamicEntityService, EntityDefinitionService};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::WorkflowKind;
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn workflow_creates_entity_with_run_uuid_as_created_by() -> anyhow::Result<()> {
    let pool = common::utils::setup_test_db().await;

    // Create admin user
    let creator_uuid = common::utils::create_test_admin_user(&pool).await?;

    // Create entity definition
    let entity_type = common::utils::unique_entity_type("test_entity");
    let _entity_def_uuid =
        common::utils::create_test_entity_definition(&pool, &entity_type).await?;

    // Create workflow
    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_adapter_arc = Arc::new(wf_adapter);
    let wf_service = WorkflowService::new(wf_adapter_arc.clone());

    let cfg = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                }
            }
        ]
    });

    let req = CreateWorkflowRequest {
        name: format!("wf-entity-test-{}", Uuid::now_v7()),
        description: Some("test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };

    let wf_uuid = wf_service.create(&req, creator_uuid).await?;

    // Create dynamic entity service
    let de_repo = DynamicEntityRepository::new(pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));
    let de_service = DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    let wf_service_with_entities =
        WorkflowService::new_with_entities(wf_adapter_arc, Arc::new(de_service));

    // Enqueue run
    let run_uuid = wf_service_with_entities.enqueue_run(wf_uuid).await?;

    // Stage raw items
    let payloads = vec![json!({
        "name": "Test User",
        "email": "test@example.com"
    })];
    wf_service_with_entities
        .stage_raw_items(wf_uuid, run_uuid, payloads)
        .await?;

    // Process staged items
    let result = wf_service_with_entities
        .process_staged_items(wf_uuid, run_uuid)
        .await;

    // Always check run logs to see what happened
    let logs = wf_service_with_entities
        .list_run_logs_paginated(run_uuid, 10, 0)
        .await?;
    eprintln!("Run logs after processing: {:?}", logs);

    if let Err(e) = result {
        eprintln!("Workflow processing failed: {:?}", e);
        return Err(e);
    }

    // Check processed/failed counts
    let (processed, failed) = result.unwrap();
    eprintln!("Processed: {}, Failed: {}", processed, failed);

    if processed == 0 && failed == 0 {
        // Check if items were actually staged
        let wf_repo_check = WorkflowRepository::new(pool.clone());
        let staged_count = wf_repo_check.count_raw_items_for_run(run_uuid).await?;
        eprintln!("Staged items count: {}", staged_count);
        return Err(anyhow::anyhow!(
            "No items were processed. Staged: {}",
            staged_count
        ));
    }

    // Wait a moment for database to sync
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify entity was created with run_uuid as created_by
    let de_repo_check = DynamicEntityRepository::new(pool.clone());
    let entities = de_repo_check
        .get_all_by_type(&entity_type, 10, 0, None)
        .await?;

    assert_eq!(
        entities.len(),
        1,
        "Expected 1 entity, found {}",
        entities.len()
    );
    let entity = &entities[0];

    // Check created_by is set to run_uuid
    let created_by_str = entity
        .field_data
        .get("created_by")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("created_by not found"))?;
    let created_by = Uuid::parse_str(created_by_str)?;
    assert_eq!(created_by, run_uuid);

    // Check updated_by is also set to run_uuid
    let updated_by_str = entity
        .field_data
        .get("updated_by")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("updated_by not found"))?;
    let updated_by = Uuid::parse_str(updated_by_str)?;
    assert_eq!(updated_by, run_uuid);

    // Verify in database directly
    let row =
        sqlx::query("SELECT created_by, updated_by FROM entities_registry WHERE entity_type = $1")
            .bind(&entity_type)
            .fetch_one(&pool)
            .await?;
    let db_created_by: Uuid = row.try_get("created_by")?;
    let db_updated_by: Option<Uuid> = row.try_get("updated_by")?;
    assert_eq!(db_created_by, run_uuid);
    assert_eq!(db_updated_by, Some(run_uuid));

    Ok(())
}

#[tokio::test]
async fn workflow_updates_entity_with_run_uuid_as_updated_by() -> anyhow::Result<()> {
    let pool = common::utils::setup_test_db().await;

    // Create admin user
    let creator_uuid = common::utils::create_test_admin_user(&pool).await?;

    // Create entity definition
    let entity_type = common::utils::unique_entity_type("test_entity");
    let _entity_def_uuid =
        common::utils::create_test_entity_definition(&pool, &entity_type).await?;

    // Create an entity first
    let de_repo = DynamicEntityRepository::new(pool.clone());
    let ed_repo = EntityDefinitionRepository::new(pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let de_service = DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service.clone()));

    let entity_uuid = Uuid::now_v7();
    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(entity_uuid.to_string()));
    field_data.insert("name".to_string(), json!("Original Name"));
    field_data.insert("email".to_string(), json!("original@example.com"));
    field_data.insert("entity_key".to_string(), json!("original-key"));
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data.insert("updated_by".to_string(), json!(creator_uuid.to_string()));

    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;
    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data,
        definition: Arc::new(entity_def),
    };

    de_service.create_entity(&entity).await?;

    // Create workflow with update mode
    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_adapter_arc = Arc::new(wf_adapter);
    let wf_service = WorkflowService::new(wf_adapter_arc.clone());

    let cfg = serde_json::json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "update",
                    "update_key": "entity_key",
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                }
            }
        ]
    });

    let req = CreateWorkflowRequest {
        name: format!("wf-update-test-{}", Uuid::now_v7()),
        description: Some("test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };

    let wf_uuid = wf_service.create(&req, creator_uuid).await?;

    // Create workflow service with entities
    let de_repo2 = DynamicEntityRepository::new(pool.clone());
    let de_adapter2 = DynamicEntityRepositoryAdapter::new(de_repo2);
    let wf_service_with_entities = WorkflowService::new_with_entities(
        wf_adapter_arc,
        Arc::new(DynamicEntityService::new(
            Arc::new(de_adapter2),
            Arc::new(ed_service),
        )),
    );

    // Enqueue run
    let run_uuid = wf_service_with_entities.enqueue_run(wf_uuid).await?;

    // Stage raw items with entity_key to find the entity
    let payloads = vec![json!({
        "entity_key": "original-key",
        "name": "Updated Name",
        "email": "updated@example.com"
    })];
    wf_service_with_entities
        .stage_raw_items(wf_uuid, run_uuid, payloads)
        .await?;

    // Process staged items
    let result = wf_service_with_entities
        .process_staged_items(wf_uuid, run_uuid)
        .await;

    // Always check run logs to see what happened
    let logs = wf_service_with_entities
        .list_run_logs_paginated(run_uuid, 10, 0)
        .await?;
    eprintln!("Run logs after processing: {:?}", logs);

    if let Err(e) = result {
        eprintln!("Workflow processing failed: {:?}", e);
        return Err(e);
    }

    // Check processed/failed counts
    let (processed, failed) = result.unwrap();
    eprintln!("Processed: {}, Failed: {}", processed, failed);

    if processed == 0 && failed == 0 {
        // Check if items were actually staged
        let wf_repo_check = WorkflowRepository::new(pool.clone());
        let staged_count = wf_repo_check.count_raw_items_for_run(run_uuid).await?;
        eprintln!("Staged items count: {}", staged_count);
        return Err(anyhow::anyhow!(
            "No items were processed. Staged: {}",
            staged_count
        ));
    }

    // Wait a moment for database to sync
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify entity was updated with run_uuid as updated_by
    let de_repo_check = DynamicEntityRepository::new(pool.clone());
    let entity_opt: Option<r_data_core_core::DynamicEntity> = de_repo_check
        .get_by_type(&entity_type, &entity_uuid, None)
        .await?;

    assert!(entity_opt.is_some());
    let entity = entity_opt.unwrap();

    // Check name was updated
    assert_eq!(
        entity
            .field_data
            .get("name")
            .and_then(|v: &serde_json::Value| v.as_str()),
        Some("Updated Name")
    );

    // Check created_by is still the original creator
    let created_by_str = entity
        .field_data
        .get("created_by")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("created_by not found"))?;
    let created_by = Uuid::parse_str(created_by_str)?;
    assert_eq!(created_by, creator_uuid);

    // Check updated_by is set to run_uuid
    let updated_by_str = entity
        .field_data
        .get("updated_by")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("updated_by not found"))?;
    let updated_by = Uuid::parse_str(updated_by_str)?;
    assert_eq!(updated_by, run_uuid);

    // Verify in database directly
    let row = sqlx::query("SELECT created_by, updated_by FROM entities_registry WHERE uuid = $1")
        .bind(entity_uuid)
        .fetch_one(&pool)
        .await?;
    let db_created_by: Uuid = row.try_get("created_by")?;
    let db_updated_by: Option<Uuid> = row.try_get("updated_by")?;
    assert_eq!(db_created_by, creator_uuid);
    assert_eq!(db_updated_by, Some(run_uuid));

    Ok(())
}
