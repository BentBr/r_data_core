#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_services::workflow::transform_execution::execute_async_transform;
use r_data_core_test_support::setup_test_db;
use r_data_core_workflow::dsl::Transform;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{dynamic_entity::DynamicEntityService, EntityDefinitionService};

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
            target_parent_uuid: Some("parent_uuid".to_string()),
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
    let _entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Create transform for non-existent entity
    let transform = Transform::ResolveEntityPath(
        r_data_core_workflow::dsl::transform::ResolveEntityPathTransform {
            target_path: "instance_path".to_string(),
            target_parent_uuid: None,
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

    // Create normalized data with non-existent license_key_id
    let mut normalized = json!({
        "license_key_id": "NON-EXISTENT"
    });

    let run_uuid = Uuid::now_v7();

    // Execute async transform - should use fallback (zero-impact resilience)
    execute_async_transform(&transform, &mut normalized, &de_service, run_uuid).await?;

    // Check that fallback path was set
    assert_eq!(
        normalized.get("instance_path").and_then(|v| v.as_str()),
        Some("/test/fallback")
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
            target_parent_uuid: Some("parent_uuid".to_string()),
            target_entity_uuid: Some("entity_uuid".to_string()),
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
