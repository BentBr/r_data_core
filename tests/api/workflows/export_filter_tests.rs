#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Tests for workflow export with different filter operators

use actix_web::test;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_services::WorkflowRepositoryAdapter;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::workflows::common::{
    create_entity_definition_with_fields, generate_entity_type, setup_app_with_entities,
};

#[actix_web::test]
async fn test_workflow_export_with_entity_source_equality_filter() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("test_export_eq");

    // Create entity definition with name, email, and status fields
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

    // Create some test entities using the repository
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service =
        r_data_core_services::EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let entity1_uuid = Uuid::now_v7();
    let entity2_uuid = Uuid::now_v7();
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let mut field_data1 = HashMap::new();
    field_data1.insert("uuid".to_string(), json!(entity1_uuid.to_string()));
    field_data1.insert("entity_key".to_string(), json!("key1"));
    field_data1.insert("path".to_string(), json!("/"));
    field_data1.insert("name".to_string(), json!("Test1"));
    field_data1.insert("email".to_string(), json!("test1@example.com"));
    field_data1.insert("status".to_string(), json!("active"));
    field_data1.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data1.insert("published".to_string(), json!(true));
    field_data1.insert("version".to_string(), json!(1));

    let entity1 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: field_data1,
        definition: Arc::new(entity_def.clone()),
    };
    entity_repo.create(&entity1).await?;

    let mut field_data2 = HashMap::new();
    field_data2.insert("uuid".to_string(), json!(entity2_uuid.to_string()));
    field_data2.insert("entity_key".to_string(), json!("key2"));
    field_data2.insert("path".to_string(), json!("/"));
    field_data2.insert("name".to_string(), json!("Test2"));
    field_data2.insert("email".to_string(), json!("test2@example.com"));
    field_data2.insert("status".to_string(), json!("inactive"));
    field_data2.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data2.insert("published".to_string(), json!(true));
    field_data2.insert("version".to_string(), json!(1));

    let entity2 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: field_data2,
        definition: Arc::new(entity_def),
    };
    entity_repo.create(&entity2).await?;

    // Create provider workflow with entity source and equality filter
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_equality_filter.json",
        &entity_type,
    )?;

    let repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("export-test-{}", Uuid::now_v7().simple()),
        description: Some("Export test".to_string()),
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

    // Should only return entities with status="active"
    assert_eq!(data.len(), 1, "Should return exactly 1 entity");
    assert_eq!(
        data[0]["status"], "active",
        "Filtered entity should have status=active"
    );

    Ok(())
}

#[actix_web::test]
async fn test_workflow_export_with_greater_than_filter() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("test_export_gt");

    // Create entity definition with price field (Integer)
    let fields = vec![FieldDefinition {
        name: "price".to_string(),
        display_name: "Price".to_string(),
        field_type: FieldType::Integer,
        required: false,
        description: Some("The price field".to_string()),
        filterable: true,
        indexed: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    }];
    let _ed_uuid = create_entity_definition_with_fields(&pool.pool, &entity_type, fields).await?;

    // Create test entities with numeric values using repository
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service =
        r_data_core_services::EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    for (i, price) in [10, 20, 5].iter().enumerate() {
        let entity_uuid = Uuid::now_v7();
        let mut field_data = HashMap::new();
        field_data.insert("uuid".to_string(), json!(entity_uuid.to_string()));
        field_data.insert("entity_key".to_string(), json!(format!("key{}", i + 1)));
        field_data.insert("path".to_string(), json!("/"));
        field_data.insert("price".to_string(), json!(*price));
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

    // Create provider workflow with greater than filter
    let config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_greater_than_filter.json",
        &entity_type,
    )?;

    let repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("export-gt-{}", Uuid::now_v7().simple()),
        description: Some("Export test GT".to_string()),
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

    // Should only return entities with price > 15
    assert_eq!(
        data.len(),
        1,
        "Should return exactly 1 entity with price > 15"
    );
    assert!(
        data[0]["price"].as_i64().unwrap_or(0) > 15,
        "Filtered entity should have price > 15"
    );

    Ok(())
}

#[actix_web::test]
async fn test_workflow_export_with_in_operator() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;

    let entity_type = generate_entity_type("test_export_in");
    // Create entity definition with category field
    let fields = vec![FieldDefinition {
        name: "category".to_string(),
        display_name: "Category".to_string(),
        field_type: FieldType::String,
        required: false,
        description: Some("The category field".to_string()),
        filterable: true,
        indexed: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    }];
    let _ed_uuid = create_entity_definition_with_fields(&pool.pool, &entity_type, fields).await?;

    // Create test entities using repository
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service =
        r_data_core_services::EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    for (i, category) in ["A", "B", "C"].iter().enumerate() {
        let entity_uuid = Uuid::now_v7();
        let mut field_data = HashMap::new();
        field_data.insert("uuid".to_string(), json!(entity_uuid.to_string()));
        field_data.insert("entity_key".to_string(), json!(format!("key{}", i + 1)));
        field_data.insert("path".to_string(), json!("/"));
        field_data.insert("category".to_string(), json!(*category));
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

    // Create provider workflow with IN operator
    let mut config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_in_operator.json",
        &entity_type,
    )?;
    // Update the IN value to be a JSON array string
    if let Some(steps) = config.get_mut("steps").and_then(|s| s.as_array_mut()) {
        if let Some(step) = steps.get_mut(0) {
            if let Some(from) = step.get_mut("from") {
                if let Some(filter) = from.get_mut("filter") {
                    if let Some(value) = filter.get_mut("value") {
                        *value = serde_json::json!("[\"A\", \"B\"]");
                    }
                }
            }
        }
    }

    let repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("export-in-{}", Uuid::now_v7().simple()),
        description: Some("Export test IN".to_string()),
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

    // Should return entities with category in ["A", "B"]
    assert_eq!(data.len(), 2, "Should return exactly 2 entities");
    let categories: Vec<String> = data
        .iter()
        .map(|e| e["category"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        categories.contains(&"A".to_string()),
        "Should include category A"
    );
    assert!(
        categories.contains(&"B".to_string()),
        "Should include category B"
    );
    assert!(
        !categories.contains(&"C".to_string()),
        "Should not include category C"
    );

    Ok(())
}

#[actix_web::test]
async fn test_workflow_export_with_boolean_filter() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let entity_type = generate_entity_type("test_export_bool");

    // Create entity definition with a boolean field
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
            name: "active".to_string(),
            display_name: "Active".to_string(),
            field_type: FieldType::Boolean,
            required: false,
            description: Some("The active field".to_string()),
            filterable: true,
            indexed: true,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        },
    ];

    let _ed_uuid = create_entity_definition_with_fields(&pool, &entity_type, fields).await?;

    // Create entities with different published and active values
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Create dynamic entity service for creating entities
    let de_repo = DynamicEntityRepository::new(pool.pool.clone());
    let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone());
    let ed_service =
        r_data_core_services::EntityDefinitionService::new_without_cache(Arc::new(ed_repo));
    let entity_def = ed_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let entity1_uuid = Uuid::now_v7();
    let entity2_uuid = Uuid::now_v7();

    // Create entity with published=true, active=true
    let mut field_data1 = HashMap::new();
    field_data1.insert("uuid".to_string(), json!(entity1_uuid.to_string()));
    field_data1.insert("entity_key".to_string(), json!("key1"));
    field_data1.insert("path".to_string(), json!("/test"));
    field_data1.insert("name".to_string(), json!("Entity 1"));
    field_data1.insert("active".to_string(), json!(true));
    field_data1.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data1.insert("published".to_string(), json!(true));
    field_data1.insert("version".to_string(), json!(1));

    let entity1 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: field_data1,
        definition: Arc::new(entity_def.clone()),
    };
    de_repo.create(&entity1).await?;

    // Create entity with published=true, active=false
    let mut field_data2 = HashMap::new();
    field_data2.insert("uuid".to_string(), json!(entity2_uuid.to_string()));
    field_data2.insert("entity_key".to_string(), json!("key2"));
    field_data2.insert("path".to_string(), json!("/test"));
    field_data2.insert("name".to_string(), json!("Entity 2"));
    field_data2.insert("active".to_string(), json!(false));
    field_data2.insert("created_by".to_string(), json!(creator_uuid.to_string()));
    field_data2.insert("published".to_string(), json!(true));
    field_data2.insert("version".to_string(), json!(1));

    let entity2 = r_data_core_core::DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: field_data2,
        definition: Arc::new(entity_def),
    };
    de_repo.create(&entity2).await?;

    // Create workflow with filter on published (system boolean field) using string "true"
    let mut config = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_equality_filter.json",
        &entity_type,
    )?;
    // Update filter to use published field with string "true"
    if let Some(steps) = config.get_mut("steps").and_then(|s| s.as_array_mut()) {
        if let Some(step) = steps.get_mut(0) {
            if let Some(from) = step.get_mut("from") {
                if let Some(from_obj) = from.as_object_mut() {
                    if let Some(filter) = from_obj.get_mut("filter") {
                        if let Some(filter_obj) = filter.as_object_mut() {
                            filter_obj.insert("field".to_string(), json!("published"));
                            filter_obj.insert("operator".to_string(), json!("="));
                            filter_obj.insert("value".to_string(), json!("true"));
                        }
                    }
                }
            }
        }
    }

    let wf_repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = r_data_core_services::WorkflowService::new(Arc::new(wf_adapter));

    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("bool-filter-test-{}", Uuid::now_v7().simple()),
        description: Some("Boolean filter test".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };

    let wf_uuid = wf_service.create(&create_req, creator_uuid).await?;

    // Call the API endpoint to export data
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Should return 200 OK");

    let body: serde_json::Value = test::read_body_json(resp).await;
    // Response is directly an array for workflow exports
    let data = body
        .as_array()
        .or_else(|| body.get("data").and_then(|v| v.as_array()))
        .ok_or_else(|| anyhow::anyhow!("Expected array in response, got: {body}"))?;

    // Should return both entities (both have published=true by default)
    assert_eq!(
        data.len(),
        2,
        "Should return exactly 2 entities with published=true"
    );

    // Test filtering on custom boolean field with string "true"
    let mut config2 = crate::api::workflows::common::load_workflow_example(
        "workflow_export_entity_equality_filter.json",
        &entity_type,
    )?;
    // Update filter to use active field with string "true"
    if let Some(steps) = config2.get_mut("steps").and_then(|s| s.as_array_mut()) {
        if let Some(step) = steps.get_mut(0) {
            if let Some(from) = step.get_mut("from") {
                if let Some(from_obj) = from.as_object_mut() {
                    if let Some(filter) = from_obj.get_mut("filter") {
                        if let Some(filter_obj) = filter.as_object_mut() {
                            filter_obj.insert("field".to_string(), json!("active"));
                            filter_obj.insert("operator".to_string(), json!("="));
                            filter_obj.insert("value".to_string(), json!("true"));
                        }
                    }
                }
            }
        }
    }

    let create_req2 = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("bool-filter-test2-{}", Uuid::now_v7().simple()),
        description: Some("Boolean filter test 2".to_string()),
        kind: r_data_core_workflow::data::WorkflowKind::Provider.to_string(),
        enabled: true,
        schedule_cron: None,
        config: config2,
        versioning_disabled: false,
    };

    let wf_uuid2 = wf_service.create(&create_req2, creator_uuid).await?;

    let req2 = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid2}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 200, "Should return 200 OK");

    let body2: serde_json::Value = test::read_body_json(resp2).await;
    // Response is directly an array for workflow exports
    let data2 = body2
        .as_array()
        .or_else(|| body2.get("data").and_then(|v| v.as_array()))
        .ok_or_else(|| anyhow::anyhow!("Expected array in response, got: {body2}"))?;

    // Should return only entity with active=true
    assert_eq!(
        data2.len(),
        1,
        "Should return exactly 1 entity with active=true"
    );
    assert_eq!(
        data2[0]["name"].as_str().unwrap(),
        "Entity 1",
        "Should return Entity 1"
    );

    Ok(())
}
