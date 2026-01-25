#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Tests for workflow export via cron jobs with entity sources

use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_persistence::{
    DynamicEntityRepository, EntityDefinitionRepository, WorkflowRepository,
};
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::workflows::common::{create_entity_definition_with_fields, generate_entity_type};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_entity_fetch_in_cron_job_with_logging() -> anyhow::Result<()> {
    let pool = r_data_core_test_support::setup_test_db().await;
    let creator_uuid = r_data_core_test_support::create_test_admin_user(&pool).await?;

    let entity_type = generate_entity_type("test_cron_fetch");
    // Create entity definition with name and status fields
    let fields = vec![
        FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("The name field".to_string()),
            filterable: true,
            indexed: true,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
        FieldDefinition {
            name: "status".to_string(),
            display_name: "Status".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The status field".to_string()),
            filterable: true,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
    ];
    let _ed_uuid = create_entity_definition_with_fields(&pool.pool, &entity_type, fields).await?;

    // Create test entities using repository
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    for (i, (status, name)) in [
        ("active", "Entity1"),
        ("active", "Entity2"),
        ("inactive", "Entity3"),
    ]
    .iter()
    .enumerate()
    {
        let entity_uuid = Uuid::now_v7();
        let mut field_data = HashMap::new();
        field_data.insert("uuid".to_string(), json!(entity_uuid.to_string()));
        field_data.insert("entity_key".to_string(), json!(format!("key{}", i + 1)));
        field_data.insert("path".to_string(), json!("/"));
        field_data.insert("name".to_string(), json!(*name));
        field_data.insert("status".to_string(), json!(*status));
        field_data.insert("created_by".to_string(), json!(creator_uuid.to_string()));
        field_data.insert("published".to_string(), json!(true));
        field_data.insert("version".to_string(), json!(1));

        let entity = r_data_core_core::DynamicEntity {
            entity_type: entity_type.clone(),
            field_data,
            definition: Arc::new(entity_def.clone()),
        };
        entity_repo.create(&entity).await?;
    }

    // Create workflow with entity source
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_cron.json",
        &entity_type,
    )?;

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_adapter_arc = Arc::new(wf_adapter);
    let wf_service = WorkflowService::new(wf_adapter_arc.clone());

    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("cron-fetch-test-{}", Uuid::now_v7().simple()),
        description: Some("Cron fetch test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: Some("0 0 * * * *".to_string()), // 6-field cron: second minute hour day month dow
        config,
        versioning_disabled: false,
    };
    let wf_uuid = wf_service.create(&create_req, creator_uuid).await?;

    // Create workflow service with entities
    let de_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));
    let de_service =
        r_data_core_services::DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    let wf_service_with_entities =
        WorkflowService::new_with_entities(wf_adapter_arc, Arc::new(de_service));

    // Enqueue a run
    let run_uuid = wf_service_with_entities.enqueue_run(wf_uuid).await?;

    // Call fetch_and_stage_from_config
    let staged_count = wf_service_with_entities
        .fetch_and_stage_from_config(wf_uuid, run_uuid)
        .await?;

    // Should have staged 2 entities (status = "active")
    assert_eq!(
        staged_count, 2,
        "Should stage 2 entities with status=active"
    );

    // Check run logs for the entity fetch message
    let logs = wf_service_with_entities
        .list_run_logs_paginated(run_uuid, 10, 0)
        .await?;

    // Debug: print all log messages
    eprintln!("All log messages:");
    for (uuid, ts, level, msg, meta) in &logs.0 {
        eprintln!("  - uuid: {uuid}, ts: {ts}, level: {level}, msg: {msg}, meta: {meta:?}");
    }

    let fetch_log = logs.0.iter().find(|(_, _, _, message, _)| {
        let msg_lower = message.to_lowercase();
        msg_lower.contains("fetched") && msg_lower.contains("entities")
    });

    assert!(
        fetch_log.is_some(),
        "Should have a log entry about fetching entities"
    );

    let (_, _, _, message, meta) = fetch_log.unwrap();
    assert!(
        message.contains("Fetched"),
        "Log message should contain 'Fetched'"
    );
    assert!(
        message.contains(&entity_type),
        "Log message should contain entity type"
    );

    // Check metadata contains entity count
    if let Some(meta_value) = meta {
        let entity_count: Option<i64> = meta_value
            .as_object()
            .and_then(|obj: &serde_json::Map<String, serde_json::Value>| obj.get("entity_count"))
            .and_then(|v: &serde_json::Value| v.as_i64());
        assert_eq!(
            entity_count,
            Some(2),
            "Metadata should contain entity_count = 2"
        );
    }

    Ok(())
}
