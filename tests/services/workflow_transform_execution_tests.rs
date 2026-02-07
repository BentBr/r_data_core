#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_services::workflow::transform_execution::execute_async_transform;
use r_data_core_test_support::setup_test_db;
use r_data_core_workflow::dsl::{DslProgram, Transform};
use serde_json::json;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{dynamic_entity::DynamicEntityService, EntityDefinitionService};

/// Load a test fixture JSON file from `.example_files/json_examples/dsl/tests/`
/// and optionally substitute `${ENTITY_TYPE}` placeholder.
fn load_test_fixture(path: &str, entity_type: Option<&str>) -> serde_json::Value {
    let content = read_to_string(format!(".example_files/json_examples/dsl/tests/{path}"))
        .expect("read test fixture");
    let content = if let Some(et) = entity_type {
        content.replace("${ENTITY_TYPE}", et)
    } else {
        content
    };
    serde_json::from_str(&content).expect("parse json")
}

// Helper to create test entity definition
async fn create_test_entity_definition(
    pool: &r_data_core_test_support::TestDatabase,
    entity_type: &str,
) -> r_data_core_core::error::Result<
    r_data_core_core::entity_definition::definition::EntityDefinition,
> {
    use r_data_core_core::{
        entity_definition::definition::EntityDefinition, field::definition::FieldDefinition,
        field::types::FieldType,
    };
    use time::OffsetDateTime;

    let entity_def = EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: format!("Test {entity_type}"),
        description: None,
        group_name: None,
        allow_children: true,
        icon: None,
        fields: vec![
            // Note: entity_key is a system field in entities_registry, don't add it as a custom field
            FieldDefinition {
                name: "license_key_id".to_string(),
                display_name: "License Key ID".to_string(),
                description: None,
                field_type: FieldType::String,
                required: false,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: r_data_core_core::entity_definition::schema::Schema::default(),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::now_v7(),
        updated_by: None,
        published: true,
        version: 1,
    };

    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    def_service
        .get_entity_definition_by_entity_type(entity_type)
        .await
}

#[tokio::test]
async fn test_execute_async_transform_resolve_entity_path() -> r_data_core_core::error::Result<()> {
    let pool = setup_test_db().await;
    let entity_type = "test_instance";
    let entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Create entity with license_key_id
    let license_key_id = "LICENSE-123";
    let entity_path = "/statistics_instance/license-123";
    let mut field_data = HashMap::new();
    field_data.insert("entity_key".to_string(), json!(Uuid::now_v7().to_string()));
    field_data.insert("path".to_string(), json!(entity_path));
    field_data.insert("license_key_id".to_string(), json!(license_key_id));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: std::sync::Arc::new(entity_def),
    };
    de_service.create_entity(&entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create transform
    let transform = Transform::ResolveEntityPath(
        r_data_core_workflow::dsl::transform::ResolveEntityPathTransform {
            target_path: "instance_path".to_string(),
            target_uuid: Some("instance_uuid".to_string()),
            entity_type: entity_type.to_string(),
            filters: {
                let mut filters = HashMap::new();
                filters.insert(
                    "license_key_id".to_string(),
                    r_data_core_workflow::dsl::StringOperand::Field {
                        field: "license_key_id".to_string(),
                    },
                );
                filters
            },
            value_transforms: None,
            fallback_path: Some("/test/fallback".to_string()),
        },
    );

    // Create normalized data with license_key_id
    let mut normalized = json!({
        "license_key_id": license_key_id
    });

    let run_uuid = Uuid::now_v7();

    // Execute async transform
    execute_async_transform(&transform, &mut normalized, &de_service, run_uuid).await?;

    // Check that path was set
    assert_eq!(
        normalized.get("instance_path").and_then(|v| v.as_str()),
        Some(entity_path)
    );

    Ok(())
}

#[tokio::test]
async fn test_execute_async_transform_resolve_entity_path_not_found(
) -> r_data_core_core::error::Result<()> {
    let pool = setup_test_db().await;
    let entity_type = "test_instance";
    let entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Create the fallback entity first (required for fallback to work)
    let fallback_path = "/test/fallback";
    let mut fallback_field_data = HashMap::new();
    fallback_field_data.insert("entity_key".to_string(), json!("fallback"));
    fallback_field_data.insert("path".to_string(), json!(fallback_path));
    fallback_field_data.insert("license_key_id".to_string(), json!("fallback"));
    fallback_field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let fallback_entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data: fallback_field_data,
        definition: std::sync::Arc::new(entity_def),
    };
    de_service.create_entity(&fallback_entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create transform for non-existent entity
    let transform = Transform::ResolveEntityPath(
        r_data_core_workflow::dsl::transform::ResolveEntityPathTransform {
            target_path: "instance_path".to_string(),
            target_uuid: Some("instance_uuid".to_string()),
            entity_type: entity_type.to_string(),
            filters: {
                let mut filters = HashMap::new();
                filters.insert(
                    "license_key_id".to_string(),
                    r_data_core_workflow::dsl::StringOperand::Field {
                        field: "license_key_id".to_string(),
                    },
                );
                filters
            },
            value_transforms: None,
            fallback_path: Some(fallback_path.to_string()),
        },
    );

    // Create normalized data with non-existent license_key_id
    let mut normalized = json!({
        "license_key_id": "NON-EXISTENT"
    });

    let run_uuid = Uuid::now_v7();

    // Execute async transform - should use fallback entity
    execute_async_transform(&transform, &mut normalized, &de_service, run_uuid).await?;

    // Check that fallback path and UUID were set
    assert_eq!(
        normalized.get("instance_path").and_then(|v| v.as_str()),
        Some(fallback_path)
    );
    assert!(
        normalized.get("instance_uuid").is_some(),
        "Should set instance_uuid to the fallback entity's UUID"
    );

    Ok(())
}

#[tokio::test]
async fn test_execute_async_transform_get_or_create_entity() -> r_data_core_core::error::Result<()>
{
    let pool = setup_test_db().await;
    let entity_type = "test_instance";
    let _entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Create transform
    let transform = Transform::GetOrCreateEntity(
        r_data_core_workflow::dsl::transform::GetOrCreateEntityTransform {
            target_path: "instance_path".to_string(),
            target_uuid: Some("entity_uuid".to_string()),
            entity_type: entity_type.to_string(),
            path_template: "/statistics_instance/{license_key_id}".to_string(),
            create_field_data: None,
            path_separator: Some("/".to_string()),
        },
    );

    // Create normalized data
    let mut normalized = json!({
        "license_key_id": "LICENSE-456"
    });

    let run_uuid = Uuid::now_v7();

    // Execute async transform
    execute_async_transform(&transform, &mut normalized, &de_service, run_uuid).await?;

    // Check that path was set
    let path = normalized.get("instance_path").and_then(|v| v.as_str());
    assert!(path.is_some());
    assert!(path.unwrap().contains("LICENSE-456"));

    // Check that entity_uuid was set
    let entity_uuid = normalized.get("entity_uuid").and_then(|v| v.as_str());
    assert!(entity_uuid.is_some());

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Execute again - should find existing entity
    let mut normalized2 = json!({
        "license_key_id": "LICENSE-456"
    });

    execute_async_transform(&transform, &mut normalized2, &de_service, run_uuid).await?;

    // Should have same path
    let path2 = normalized2.get("instance_path").and_then(|v| v.as_str());
    assert_eq!(path, path2);

    Ok(())
}

/// Test that step-by-step execution properly chains async transforms (`ResolveEntityPath`)
/// with sync transforms (`BuildPath`) that depend on their results.
///
/// This is the key test for the fix: `BuildPath` in step 1 should be able to use
/// `instance_path` that was set by `ResolveEntityPath` in step 0.
#[tokio::test]
#[allow(clippy::too_many_lines)] // Test logic is cohesive and easier to follow as one function
async fn test_step_by_step_execution_resolve_then_build_path() -> r_data_core_core::error::Result<()>
{
    let pool = setup_test_db().await;
    let entity_type = "test_instance_stepwise";
    let entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Create entity with license_key_id
    let license_key_id = "STEPWISE-TEST-123";
    let entity_path = "/instances/stepwise-test";
    let mut field_data = HashMap::new();
    field_data.insert("entity_key".to_string(), json!(Uuid::now_v7().to_string()));
    field_data.insert("path".to_string(), json!(entity_path));
    field_data.insert("license_key_id".to_string(), json!(license_key_id));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    let entity = r_data_core_core::DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: std::sync::Arc::new(entity_def),
    };
    de_service.create_entity(&entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create 2-step workflow:
    // Step 0: ResolveEntityPath - finds entity by license_key_id, sets instance_path
    // Step 1: BuildPath - uses instance_path to build submission_path
    let workflow_config = load_test_fixture(
        "test_stepwise_resolve_then_build_path.json",
        Some(entity_type),
    );

    let program = DslProgram::from_config(&workflow_config)?;
    program.validate()?;

    // Input payload
    let payload = json!({
        "license_key_id": license_key_id,
        "submission_id": "sub-001"
    });

    let run_uuid = Uuid::now_v7();

    // ========== Step-by-step execution (simulating item_processing flow) ==========

    // Step 0: Prepare
    let (mut normalized_0, transform_0) = program.prepare_step(0, &payload, None)?;

    // Step 0: Execute async transform (ResolveEntityPath)
    assert!(matches!(transform_0, Transform::ResolveEntityPath(_)));
    execute_async_transform(transform_0, &mut normalized_0, &de_service, run_uuid).await?;

    // Verify instance_path was set by async transform
    let instance_path_after_async = normalized_0
        .get("instance_path")
        .and_then(|v| v.as_str())
        .expect("instance_path should be set by ResolveEntityPath");
    assert_eq!(instance_path_after_async, entity_path);

    // Step 0: Finalize
    let (_to_def_0, produced_0) = program.finalize_step(0, &normalized_0)?;
    let next_step_input = program.get_next_step_input(0, &normalized_0, &produced_0)?;

    // Verify instance_path is passed to next step
    assert_eq!(
        next_step_input
            .get("instance_path")
            .and_then(|v| v.as_str()),
        Some(entity_path)
    );

    // Step 1: Prepare (reads from previous step output)
    let (mut normalized_1, transform_1) =
        program.prepare_step(1, &payload, Some(&next_step_input))?;

    // Verify instance_path is available in normalized data
    assert_eq!(
        normalized_1.get("instance_path").and_then(|v| v.as_str()),
        Some(entity_path),
        "instance_path should be available in step 1's normalized data"
    );

    // Step 1: Apply BuildPath transform (this is the key test!)
    assert!(matches!(transform_1, Transform::BuildPath(_)));
    DslProgram::apply_build_path(1, transform_1, &mut normalized_1)?;

    // Verify submission_path was built correctly
    let submission_path = normalized_1
        .get("submission_path")
        .and_then(|v| v.as_str())
        .expect("submission_path should be set by BuildPath");
    assert_eq!(
        submission_path,
        format!("{entity_path}/submission"),
        "BuildPath should use instance_path from previous async transform"
    );

    // Step 1: Finalize
    let (_to_def_1, produced_1) = program.finalize_step(1, &normalized_1)?;

    // Final verification: produced output contains the built path
    assert_eq!(
        produced_1.get("submission_path").and_then(|v| v.as_str()),
        Some(format!("{entity_path}/submission").as_str())
    );

    Ok(())
}

/// Test that `BuildPath` fails gracefully when async transform didn't set the required field
/// (e.g., entity not found and no fallback)
#[tokio::test]
async fn test_step_by_step_execution_build_path_fails_without_async_result(
) -> r_data_core_core::error::Result<()> {
    // Create a workflow that expects instance_path but doesn't have it
    let workflow_config = load_test_fixture("test_stepwise_build_path_missing_field.json", None);

    let program = DslProgram::from_config(&workflow_config)?;
    program.validate()?;

    // Input without instance_path
    let payload = json!({
        "license_key_id": "TEST-123"
    });

    // Step 0
    let (normalized_0, _transform_0) = program.prepare_step(0, &payload, None)?;
    let (_to_def_0, produced_0) = program.finalize_step(0, &normalized_0)?;
    let next_step_input = program.get_next_step_input(0, &normalized_0, &produced_0)?;

    // Step 1: Prepare
    let (mut normalized_1, transform_1) =
        program.prepare_step(1, &payload, Some(&next_step_input))?;

    // Step 1: BuildPath should fail because instance_path is missing
    let result = DslProgram::apply_build_path(1, transform_1, &mut normalized_1);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("instance_path") && err_msg.contains("not found"),
        "Error should mention missing instance_path: {err_msg}"
    );

    Ok(())
}
