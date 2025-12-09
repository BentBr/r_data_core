#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Tests for workflow export security (SQL injection prevention)

use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::workflows::common::{
    create_entity_definition_with_fields, generate_entity_type, setup_app_with_entities,
};

#[actix_web::test]
async fn test_workflow_export_sql_injection_prevention() -> anyhow::Result<()> {
    let (_app, pool, _token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("test_sql_inj");

    // Create entity definition with name and value fields
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
            name: "value".to_string(),
            display_name: "Value".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The value field".to_string()),
            filterable: true,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
    ];
    let _ed_uuid = create_entity_definition_with_fields(&pool.pool, &entity_type, fields).await?;

    // Create a test entity using repository
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service =
        r_data_core_services::EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let entity1_uuid = Uuid::now_v7();
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(entity1_uuid.to_string()));
    field_data.insert("entity_key".to_string(), json!("key1"));
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert("name".to_string(), json!("Test"));
    field_data.insert("value".to_string(), json!("safe"));
    field_data.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data.insert("published".to_string(), json!(true));
    field_data.insert("version".to_string(), json!(1));

    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data,
        definition: Arc::new(entity_def),
    };
    entity_repo.create(&entity).await?;

    // Test SQL injection in filter field name
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_sql_injection_field.json",
        &entity_type,
    )?;

    let wf_repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("sql-inj-test-{}", Uuid::now_v7().simple()),
        description: Some("SQL injection test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };

    // This should fail validation because the field name is invalid
    let result = wf_service.create(&create_req, creator_uuid).await;
    assert!(
        result.is_err(),
        "Should reject invalid field name with SQL injection attempt"
    );

    // Test SQL injection in filter value - should be safely parameterized
    let config2 = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_sql_injection_value.json",
        &entity_type,
    )?;

    let create_req2 = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("sql-inj-value-{}", Uuid::now_v7().simple()),
        description: Some("SQL injection value test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config: config2,
        versioning_disabled: false,
    };

    // This should succeed because the value is parameterized
    let wf_uuid = wf_service.create(&create_req2, creator_uuid).await?;

    // Verify the workflow exists and tables are still intact
    let workflow_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflows WHERE uuid = $1)")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert!(workflow_exists, "Workflow should be created");

    // Verify entities_registry table still exists (not dropped)
    let entities_exist: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'entities_registry')",
    )
    .fetch_one(&pool.pool)
    .await?;
    assert!(entities_exist, "Entities registry table should still exist");

    // Test SQL injection in operator - should be rejected
    let config3 = json!({
        "steps": [
            {
                "from": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "filter": {
                        "field": "name",
                        "operator": "= OR 1=1 --",
                        "value": "Test"
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }
        ]
    });

    let create_req3 = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("sql-inj-op-{}", Uuid::now_v7().simple()),
        description: Some("SQL injection operator test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config: config3,
        versioning_disabled: false,
    };

    // This should fail validation because the operator is invalid
    let result3 = wf_service.create(&create_req3, creator_uuid).await;
    assert!(
        result3.is_err(),
        "Should reject invalid operator with SQL injection attempt"
    );

    Ok(())
}
