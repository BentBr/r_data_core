#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Tests for workflow export mapping fallback behavior

use actix_web::test;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_persistence::DynamicEntityRepository;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::workflows::common::{
    create_entity_definition_with_fields, generate_entity_type, setup_app_with_entities,
};

#[actix_web::test]
async fn test_workflow_export_empty_mapping_fallback() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("test_empty_map");

    // Create entity definition with name, email, and age fields
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
            name: "email".to_string(),
            display_name: "Email".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("The email field".to_string()),
            filterable: true,
            indexed: true,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
        FieldDefinition {
            name: "age".to_string(),
            display_name: "Age".to_string(),
            field_type: FieldType::Integer,
            required: false,
            description: Some("The age field".to_string()),
            filterable: true,
            indexed: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
    ];
    let _ed_uuid = create_entity_definition_with_fields(&pool.pool, &entity_type, fields).await?;

    // Create a test entity with multiple fields using repository
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
    field_data.insert("email".to_string(), json!("test@example.com"));
    field_data.insert("age".to_string(), json!(30));
    field_data.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data.insert("published".to_string(), json!(true));
    field_data.insert("version".to_string(), json!(1));

    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data,
        definition: Arc::new(entity_def),
    };
    entity_repo.create(&entity).await?;

    // Create provider workflow with empty mapping (should pass through all fields)
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_empty_mapping.json",
        &entity_type,
    )?;

    let repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("export-empty-map-{}", Uuid::now_v7().simple()),
        description: Some("Empty mapping test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Test GET endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Should return 200 OK");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let data = body.as_array().expect("Response should be an array");

    assert_eq!(data.len(), 1, "Should return exactly 1 entity");

    // All fields should be present (empty mapping = pass through all fields)
    assert!(data[0]["name"].is_string(), "Should include name field");
    assert!(data[0]["email"].is_string(), "Should include email field");
    assert!(data[0]["age"].is_number(), "Should include age field");

    Ok(())
}
