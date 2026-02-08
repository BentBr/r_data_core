#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_api::admin::workflows::models::CreateWorkflowRequest;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::field::{FieldDefinition, FieldType};
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::adapters::DynamicEntityRepositoryAdapter;
use r_data_core_services::adapters::EntityDefinitionRepositoryAdapter;
use r_data_core_services::{DynamicEntityService, EntityDefinitionService};
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use r_data_core_workflow::data::adapters::format::FormatHandler;
use r_data_core_workflow::data::WorkflowKind;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// Import common test utilities for loading workflow examples
mod api {
    pub mod workflows {
        pub mod common {
            pub fn load_workflow_example(
                filename: &str,
                entity_type: &str,
            ) -> anyhow::Result<serde_json::Value> {
                use std::fs;
                let path = format!(".example_files/json_examples/dsl/{filename}");
                let content = fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;
                let content = content.replace("${ENTITY_TYPE}", entity_type);
                serde_json::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse {path}: {e}"))
            }
        }
    }
}

/// Test that consecutive imports of the same file produce identical outcomes
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_consecutive_imports_produce_identical_outcomes() -> anyhow::Result<()> {
    // Setup test database
    let pool = setup_test_db().await;

    // Create entity definition (must start with a letter)
    let entity_type = format!("TestCustomer{}", Uuid::now_v7().simple());
    let ed_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));

    let mut entity_def = EntityDefinition {
        entity_type: entity_type.clone(),
        display_name: format!("{entity_type} Class"),
        description: Some(format!("Test description for {entity_type}")),
        published: true,
        ..Default::default()
    };

    // Add required fields
    let mut fields = Vec::new();

    let email_field = FieldDefinition {
        name: "email".to_string(),
        display_name: "Email".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("The email field".to_string()),
        filterable: true,
        indexed: true,
        unique: false,
        default_value: None,
        validation: r_data_core_core::field::FieldValidation::default(),
        ui_settings: r_data_core_core::field::ui::UiSettings::default(),
        constraints: std::collections::HashMap::new(),
    };
    fields.push(email_field);

    let name_field = FieldDefinition {
        name: "name".to_string(),
        display_name: "Name".to_string(),
        field_type: FieldType::String,
        required: false,
        description: Some("The name field".to_string()),
        filterable: true,
        indexed: true,
        unique: false,
        default_value: None,
        validation: r_data_core_core::field::FieldValidation::default(),
        ui_settings: r_data_core_core::field::ui::UiSettings::default(),
        constraints: std::collections::HashMap::new(),
    };
    fields.push(name_field);

    entity_def.fields = fields;
    let _ed_uuid = ed_service
        .create_entity_definition(&entity_def)
        .await
        .expect("create entity definition");

    // Create workflow with DSL configuration
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    // Create a test admin user
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create test admin user");

    let workflow_name = format!("test-wf-{}", Uuid::now_v7().simple());
    let mut workflow_config =
        api::workflows::common::load_workflow_example("workflow_csv_to_entity.json", &entity_type)?;
    // Update path to /test
    if let Some(steps) = workflow_config
        .get_mut("steps")
        .and_then(|s| s.as_array_mut())
    {
        if let Some(step) = steps.get_mut(0) {
            if let Some(to) = step.get_mut("to") {
                if let Some(to_obj) = to.as_object_mut() {
                    to_obj.insert("path".to_string(), serde_json::json!("/test"));
                }
            }
        }
    }

    let req = CreateWorkflowRequest {
        name: workflow_name.clone(),
        description: Some("test consecutive imports".into()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: workflow_config,
        versioning_disabled: false,
    };
    let wf_uuid = wf_service
        .create(&req, creator_uuid)
        .await
        .expect("create workflow");

    // Create DynamicEntity service
    let de_repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let de_service = DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service.clone()));

    // Create WorkflowService with entity service
    let wf_adapter_entities =
        WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.pool.clone()));
    let wf_service_with_entities = WorkflowService::new_with_entities(
        Arc::new(wf_adapter_entities),
        Arc::new(de_service.clone()),
    );

    // Test data - CSV content with email and name
    let csv_data = "email,name\ntest@example.com,Test User\ntest2@example.com,Test User 2";
    let format_cfg = json!({
        "has_header": true,
        "delimiter": ","
    });
    let payloads = r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new()
        .parse(csv_data.as_bytes(), &format_cfg)
        .expect("parse CSV");

    // Run import multiple times (3-5 times) and verify identical outcomes
    let num_runs = 5;
    let mut results = Vec::new();

    for run_idx in 0..num_runs {
        // Create a new run for each import
        let trigger_id = Uuid::now_v7();
        let wf_repo_run = WorkflowRepository::new(pool.pool.clone());
        let run_uuid = wf_repo_run
            .insert_run_queued(wf_uuid, trigger_id)
            .await
            .expect("insert queued run");

        // Stage the same data
        let staged = wf_repo_run
            .insert_raw_items(wf_uuid, run_uuid, payloads.clone())
            .await
            .expect("stage raw items");

        // Process staged items
        let (processed, failed) = wf_service_with_entities
            .process_staged_items(wf_uuid, run_uuid)
            .await
            .expect("process staged items");

        // Get run logs to check error messages
        let logs = sqlx::query!(
            r#"SELECT level, message FROM workflow_run_logs WHERE run_uuid = $1 ORDER BY ts ASC"#,
            run_uuid
        )
        .fetch_all(&pool.pool)
        .await
        .expect("get run logs");

        let error_messages: Vec<String> = logs
            .iter()
            .filter(|log| log.level == "error")
            .map(|log| log.message.clone())
            .collect();

        results.push((staged, processed, failed, error_messages.clone()));

        // Clean up entities created in this run (except for the last run)
        if run_idx < num_runs - 1 {
            // Delete entities created in this run to allow next run to test duplicate behavior
            let entities = de_service
                .list_entities(&entity_type, 100, 0, None)
                .await
                .expect("list entities");
            for entity in entities {
                // Extract UUID from field_data
                if let Some(serde_json::Value::String(uuid_str)) = entity.field_data.get("uuid") {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        let _ = de_service.delete_entity(&entity_type, &uuid).await;
                    }
                }
            }
        }
    }

    // Verify all runs produced identical outcomes
    let first_result = &results[0];
    for (idx, result) in results.iter().enumerate() {
        assert_eq!(
            result.0, first_result.0,
            "Run {idx}: Staged items count differs from first run"
        );
        assert_eq!(
            result.1, first_result.1,
            "Run {idx}: Processed items count differs from first run"
        );
        assert_eq!(
            result.2, first_result.2,
            "Run {idx}: Failed items count differs from first run"
        );
        assert_eq!(
            result.3, first_result.3,
            "Run {}: Error messages differ from first run. Got: {:?}, Expected: {:?}",
            idx, result.3, first_result.3
        );
    }

    // Verify that the produced data is consistent
    // For the first run, entities should be created successfully
    // For subsequent runs, we should get duplicate key errors (if entities weren't deleted)
    // But the error messages should be identical across runs

    // Clean up
    let _ = wf_service.delete(wf_uuid).await;
    let _ = ed_service.delete_entity_definition(&entity_def.uuid).await;
    Ok(())
}
