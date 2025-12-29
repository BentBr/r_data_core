#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use r_data_core_core::{
    entity_definition::definition::EntityDefinition, field::definition::FieldDefinition,
    field::types::FieldType,
};
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::{setup_test_db, unique_entity_type};

// Helper function to create a test entity definition struct for dynamic entities
fn create_test_entity_definition_struct() -> EntityDefinition {
    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: "test_entity".to_string(),
        display_name: "Test Entity".to_string(),
        description: Some("Test Description".to_string()),
        group_name: Some("Test Group".to_string()),
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: Some("Person's name".to_string()),
                field_type: FieldType::String,
                required: true,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "age".to_string(),
                display_name: "Age".to_string(),
                description: Some("Person's age".to_string()),
                field_type: FieldType::Integer,
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
        updated_by: Some(Uuid::now_v7()),
        published: false,
        version: 1,
    }
}

// Helper function to create a test dynamic entity
fn create_test_dynamic_entity(entity_definition: &EntityDefinition) -> DynamicEntity {
    let unique_key = Uuid::now_v7();
    let mut field_data = HashMap::new();
    field_data.insert("name".to_string(), json!("John Doe"));
    field_data.insert("age".to_string(), json!(30));
    field_data.insert("entity_key".to_string(), json!(unique_key.to_string()));
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    DynamicEntity {
        entity_type: entity_definition.entity_type.clone(),
        field_data,
        definition: Arc::new(entity_definition.clone()),
    }
}

// Helper function to create a test dynamic entity with specific path
fn create_test_dynamic_entity_with_uuid_and_path(
    entity_definition: &EntityDefinition,
    path: &str,
) -> DynamicEntity {
    let unique_key = Uuid::now_v7();
    let mut field_data = HashMap::new();
    field_data.insert("name".to_string(), json!("John Doe"));
    field_data.insert("age".to_string(), json!(30));
    field_data.insert("entity_key".to_string(), json!(unique_key.to_string()));
    field_data.insert("path".to_string(), json!(path));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    DynamicEntity {
        entity_type: entity_definition.entity_type.clone(),
        field_data,
        definition: Arc::new(entity_definition.clone()),
    }
}

/// Test `query_by_parent` method
#[tokio::test]
async fn test_query_by_parent() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create a test entity definition with unique name
    let entity_type = unique_entity_type("test_entity");
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.entity_type = entity_type.clone();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let parent = create_test_dynamic_entity(&created_def);
    let parent_uuid = repo.create(&parent).await?;

    let child1_key = Uuid::now_v7();
    let child2_key = Uuid::now_v7();
    let child1_path = format!("/{child1_key}");
    let child2_path = format!("/{child2_key}");

    let mut child1 = create_test_dynamic_entity_with_uuid_and_path(&created_def, &child1_path);
    child1.set("parent_uuid", parent_uuid.to_string())?;
    let child1_uuid = repo.create(&child1).await?;

    let mut child2 = create_test_dynamic_entity_with_uuid_and_path(&created_def, &child2_path);
    child2.set("parent_uuid", parent_uuid.to_string())?;
    let child2_uuid = repo.create(&child2).await?;

    // Test query by parent_uuid - should find both children
    let children = repo
        .query_by_parent(&entity_type, parent_uuid, 100, 0)
        .await?;
    assert_eq!(children.len(), 2, "Should find 2 children");

    // Verify we got the correct children
    // The main test is that we get 2 children with the correct parent_uuid
    assert_eq!(children.len(), 2, "Should find 2 children");

    // Verify parent_uuid relationship - this is the core functionality being tested
    // The SQL query filters by parent_uuid, so if we get 2 results, they must have the correct parent
    for child in &children {
        // parent_uuid should be extractable from field_data
        let child_parent = child
            .get::<Uuid>("parent_uuid")
            .expect("parent_uuid should be in field_data for query_by_parent results");
        assert_eq!(
            child_parent, parent_uuid,
            "Child should have correct parent_uuid"
        );
    }

    // Verify we got the correct children by checking they can be retrieved by their UUIDs
    // This confirms the query returned the right entities
    let child1_retrieved: Option<DynamicEntity> =
        repo.get_by_type(&entity_type, &child1_uuid, None).await?;
    let child2_retrieved: Option<DynamicEntity> =
        repo.get_by_type(&entity_type, &child2_uuid, None).await?;

    assert!(
        child1_retrieved.is_some(),
        "Child1 should exist and be retrievable"
    );
    assert!(
        child2_retrieved.is_some(),
        "Child2 should exist and be retrievable"
    );

    // Verify the children we got have the correct parent by checking the database directly
    // This is a more reliable way to verify than extracting from field_data
    // We can access the pool from the repo to verify the parent_uuid in the database
    let pool = &repo.pool;

    for child_uuid in [child1_uuid, child2_uuid] {
        let parent_check: Option<Uuid> =
            sqlx::query_scalar("SELECT parent_uuid FROM entities_registry WHERE uuid = $1")
                .bind(child_uuid)
                .fetch_optional(pool)
                .await?;

        assert_eq!(
            parent_check,
            Some(parent_uuid),
            "Child {child_uuid} should have parent_uuid {parent_uuid}"
        );
    }

    Ok(())
}

/// Test `query_by_path` method
#[tokio::test]
async fn test_query_by_path() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create a test entity definition with unique name
    let entity_type = unique_entity_type("test_entity");
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.entity_type = entity_type.clone();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let parent_path = "/test";
    let parent = create_test_dynamic_entity(&created_def);
    let mut parent_entity = parent;
    parent_entity.set("path", parent_path.to_string())?;
    let parent_uuid = repo.create(&parent_entity).await?;

    // Create another entity at the same path
    let sibling = create_test_dynamic_entity(&created_def);
    let mut sibling_entity = sibling;
    sibling_entity.set("path", parent_path.to_string())?;
    let sibling_uuid = repo.create(&sibling_entity).await?;

    // Create an entity at a different path
    let other_path = "/other";
    let other = create_test_dynamic_entity(&created_def);
    let mut other_entity = other;
    other_entity.set("path", other_path.to_string())?;
    let other_uuid = repo.create(&other_entity).await?;

    // Test query by path - should find entities at that path
    let path_entities = repo
        .query_by_path(&entity_type, parent_path, 100, 0)
        .await?;
    assert_eq!(path_entities.len(), 2, "Should find 2 entities at the path");

    // Verify we got the correct entities
    let found_uuids: Vec<Uuid> = path_entities
        .iter()
        .map(|e| e.get::<Uuid>("uuid").unwrap())
        .collect();
    assert!(
        found_uuids.contains(&parent_uuid),
        "Should include parent entity"
    );
    assert!(
        found_uuids.contains(&sibling_uuid),
        "Should include sibling entity"
    );
    assert!(
        !found_uuids.contains(&other_uuid),
        "Should not include entity from different path"
    );

    // Verify all returned entities have the correct path
    for entity in &path_entities {
        let entity_path = entity
            .field_data
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(
            entity_path, parent_path,
            "All returned entities should have the queried path"
        );
    }

    Ok(())
}

/// Test `has_children` method
#[tokio::test]
async fn test_has_children() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create a test entity definition with unique name
    let entity_type = unique_entity_type("test_has_children");
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.entity_type = entity_type.clone();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    let parent = create_test_dynamic_entity(&created_def);
    let parent_uuid = repo.create(&parent).await?;

    // Initially should have no children
    let has_children = repo.has_children(&parent_uuid).await?;
    assert!(!has_children, "Parent should initially have no children");

    // Create child entity
    let mut child = create_test_dynamic_entity(&created_def);
    child.set("parent_uuid", parent_uuid.to_string())?;
    repo.create(&child).await?;

    // Now should have children
    let has_children = repo.has_children(&parent_uuid).await?;
    assert!(
        has_children,
        "Parent should have children after creating child"
    );

    Ok(())
}
