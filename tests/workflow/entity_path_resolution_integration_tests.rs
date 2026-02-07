#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::{
    dynamic_entity::DynamicEntityService, workflow::entity_persistence::resolve_entity_path,
    EntityDefinitionService,
};
use r_data_core_test_support::setup_test_db;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Helper to create test entity definition
async fn create_test_entity_definition(
    pool: &r_data_core_test_support::TestDatabase,
    entity_type: &str,
) -> Result<r_data_core_core::entity_definition::definition::EntityDefinition> {
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
            FieldDefinition {
                name: "admin_uri".to_string(),
                display_name: "Admin URI".to_string(),
                description: None,
                field_type: FieldType::String,
                required: false,
                indexed: false,
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

// Helper to create test entity
fn create_test_entity(
    entity_def: &r_data_core_core::entity_definition::definition::EntityDefinition,
    license_key_id: Option<&str>,
    path: &str,
) -> r_data_core_core::DynamicEntity {
    use std::collections::HashMap;
    use std::sync::Arc;

    let mut field_data = HashMap::new();
    // entity_key is a system field, will be set automatically
    field_data.insert("path".to_string(), json!(path));
    // entity_key is required - generate one if not provided
    field_data
        .entry("entity_key".to_string())
        .or_insert_with(|| json!(Uuid::now_v7().to_string()));
    if let Some(license_id) = license_key_id {
        field_data.insert("license_key_id".to_string(), json!(license_id));
    }
    field_data.insert("admin_uri".to_string(), json!("https://example.com"));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    r_data_core_core::DynamicEntity {
        entity_type: entity_def.entity_type.clone(),
        field_data,
        definition: Arc::new(entity_def.clone()),
    }
}

#[tokio::test]
async fn test_resolve_entity_path_found() -> Result<()> {
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
    let entity = create_test_entity(&entity_def, Some(license_key_id), entity_path);
    let _entity_uuid = de_service.create_entity(&entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Resolve by license_key_id
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!(license_key_id));

    let result = resolve_entity_path(entity_type, &filters, None, None, &de_service).await?;

    assert!(result.is_some());
    let (path, entity_uuid) = result.unwrap();
    assert_eq!(path, entity_path);
    assert!(entity_uuid.is_some()); // Entity was found

    Ok(())
}

#[tokio::test]
async fn test_resolve_entity_path_not_found_uses_fallback() -> Result<()> {
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

    // Create the fallback entity (required for fallback to work)
    let fallback_path = "/test/fallback";
    let fallback_entity = create_test_entity(&entity_def, Some("fallback"), fallback_path);
    let fallback_uuid = de_service.create_entity(&fallback_entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try to resolve non-existent entity
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!("NON-EXISTENT"));

    let result = resolve_entity_path(
        entity_type,
        &filters,
        None,
        Some(fallback_path),
        &de_service,
    )
    .await?;

    // Should return fallback path and UUID when entity not found
    assert!(result.is_some());
    let (path, entity_uuid) = result.unwrap();
    assert_eq!(path, fallback_path);
    assert!(
        entity_uuid.is_some(),
        "Should return the fallback entity's UUID"
    );
    assert_eq!(entity_uuid.unwrap(), fallback_uuid);

    Ok(())
}

#[tokio::test]
async fn test_resolve_entity_path_with_value_transform() -> Result<()> {
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

    // Create entity with lowercase license_key_id
    let license_key_id = "license-123";
    let entity_path = "/statistics_instance/license-123";
    let entity = create_test_entity(&entity_def, Some(license_key_id), entity_path);
    de_service.create_entity(&entity).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Resolve with uppercase input (should be transformed to lowercase)
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!("LICENSE-123"));

    let mut transforms = HashMap::new();
    transforms.insert("license_key_id".to_string(), "lowercase".to_string());

    let result = resolve_entity_path(
        entity_type,
        &filters,
        Some(&transforms),
        Some("/test/fallback"),
        &de_service,
    )
    .await?;

    assert!(result.is_some());
    let (path, entity_uuid) = result.unwrap();
    assert_eq!(path, entity_path);
    assert!(entity_uuid.is_some()); // Entity was found

    Ok(())
}

#[tokio::test]
async fn test_resolve_entity_path_database_error_uses_fallback() -> Result<()> {
    // This test verifies that database errors don't cause failures
    // In a real scenario, we'd mock the service to return an error
    // For now, we test with a non-existent entity type which will cause an error
    let pool = setup_test_db().await;

    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let de_service = DynamicEntityService::new(
        Arc::new(repo)
            as Arc<dyn r_data_core_persistence::DynamicEntityRepositoryTrait + Send + Sync>,
        Arc::new(EntityDefinitionService::new_without_cache(Arc::new(
            EntityDefinitionRepository::new(pool.pool.clone()),
        ))),
    );

    // Try to resolve with non-existent entity type
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!("TEST"));

    // Test with non-existent entity type - should return error, not fallback
    let result = resolve_entity_path("non_existent_type", &filters, None, None, &de_service).await;

    // Should return error for database issues, not use fallback
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_find_one_by_filters() -> Result<()> {
    let pool = setup_test_db().await;
    let entity_type = "test_instance";
    let entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create multiple entities
    let license_key_id1 = "LICENSE-1";
    let license_key_id2 = "LICENSE-2";
    let entity1 = create_test_entity(&entity_def, Some(license_key_id1), "/instance-1");
    let entity2 = create_test_entity(&entity_def, Some(license_key_id2), "/instance-2");

    repo.create(&entity1).await?;
    repo.create(&entity2).await?;

    // Wait for database
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Find by filter
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!(license_key_id1));

    let result = repo.find_one_by_filters(entity_type, &filters).await?;

    assert!(result.is_some());
    let found = result.unwrap();
    assert_eq!(
        found
            .field_data
            .get("license_key_id")
            .and_then(|v| v.as_str()),
        Some(license_key_id1)
    );

    Ok(())
}

#[tokio::test]
async fn test_find_one_by_filters_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    let entity_type = "test_instance";
    let _entity_def = create_test_entity_definition(&pool, entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Try to find non-existent entity
    let mut filters = HashMap::new();
    filters.insert("license_key_id".to_string(), json!("NON-EXISTENT"));

    let result = repo.find_one_by_filters(entity_type, &filters).await?;

    assert!(result.is_none());

    Ok(())
}
