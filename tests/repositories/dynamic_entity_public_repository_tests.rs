#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

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
use r_data_core_persistence::DynamicEntityPublicRepository;
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_test_support::{setup_test_db, unique_entity_type};

// Helper function to create a test entity definition
async fn create_test_entity_definition(
    pool: &sqlx::PgPool,
    entity_type: &str,
) -> Result<EntityDefinition> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    let entity_def = EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: entity_type.to_string(),
        display_name: format!("Test {}", entity_type),
        description: Some(format!("Test description for {}", entity_type)),
        group_name: None,
        allow_children: true,
        icon: None,
        fields: vec![FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            description: Some("Name field".to_string()),
            field_type: FieldType::String,
            required: true,
            indexed: true,
            filterable: true,
            default_value: None,
            validation: Default::default(),
            ui_settings: Default::default(),
            constraints: HashMap::new(),
        }],
        schema: Default::default(),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::now_v7(),
        updated_by: Some(Uuid::now_v7()),
        published: true,
        version: 1,
    };

    let def_repo = EntityDefinitionRepository::new(pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    def_service
        .get_entity_definition_by_entity_type(entity_type)
        .await
        .map_err(|e| {
            eprintln!("Error getting entity definition: {e:?}");
            e
        })
}

// Helper function to create a test dynamic entity
fn create_test_dynamic_entity(
    entity_def: &EntityDefinition,
    name: &str,
    path: &str,
    entity_key: &str,
) -> DynamicEntity {
    let uuid = Uuid::now_v7();
    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("name".to_string(), json!(name));
    field_data.insert("entity_key".to_string(), json!(entity_key));
    field_data.insert("path".to_string(), json!(path));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    DynamicEntity {
        entity_type: entity_def.entity_type.clone(),
        field_data,
        definition: Arc::new(entity_def.clone()),
    }
}

/// Test listing available entity types
#[tokio::test]
async fn test_list_available_entity_types() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.clone());

    // Create multiple entity definitions with unique names
    let entity_type1 = unique_entity_type("test_type_1");
    let entity_type2 = unique_entity_type("test_type_2");
    let entity_def1 = create_test_entity_definition(&pool, &entity_type1).await?;
    let _entity_def2 = create_test_entity_definition(&pool, &entity_type2).await?;

    // Create entities for counting
    let repo = DynamicEntityRepository::new(pool.clone());
    let entity1 =
        create_test_dynamic_entity(&entity_def1, "Entity 1", "/", &Uuid::now_v7().to_string());
    let entity2 =
        create_test_dynamic_entity(&entity_def1, "Entity 2", "/", &Uuid::now_v7().to_string());
    repo.create(&entity1).await?;
    repo.create(&entity2).await?;

    // Wait a bit for view/table creation to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // List available entity types
    let entity_types = pub_repo.list_available_entity_types().await?;

    // Should include both entity types
    assert!(
        entity_types.iter().any(|et| et.name == entity_type1),
        "Should include entity_type1"
    );
    assert!(
        entity_types.iter().any(|et| et.name == entity_type2),
        "Should include entity_type2"
    );

    // Check entity counts
    let type1 = entity_types
        .iter()
        .find(|et| et.name == entity_type1)
        .unwrap();
    assert_eq!(type1.entity_count, 2, "entity_type1 should have 2 entities");

    Ok(())
}

/// Test browsing by path
#[tokio::test]
async fn test_browse_by_path() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_browse");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    // Create entities in a hierarchy
    let repo = DynamicEntityRepository::new(pool.clone());

    // Root level entities - use entity_key as the key
    let root1_uuid = Uuid::now_v7();
    let root2_uuid = Uuid::now_v7();
    let root1_key = format!("root1-{}", root1_uuid);
    let root2_key = format!("root2-{}", root2_uuid);
    let root1 = create_test_dynamic_entity(&entity_def, "Root 1", "/", &root1_key);
    let root2 = create_test_dynamic_entity(&entity_def, "Root 2", "/", &root2_key);
    repo.create(&root1).await?;
    repo.create(&root2).await?;

    // Get root1 UUID and path for child
    let root1_uuid = root1.get::<Uuid>("uuid")?;
    let root1_path = format!("/{}", root1_key);

    // Child entities
    let child1_uuid = Uuid::now_v7();
    let child1_key = format!("child1-{}", child1_uuid);
    let mut child1 = create_test_dynamic_entity(&entity_def, "Child 1", &root1_path, &child1_key);
    child1.set("parent_uuid", root1_uuid.to_string())?;
    repo.create(&child1).await?;

    // Browse root path
    let (nodes, total) = pub_repo.browse_by_path("/", 100, 0).await?;

    // Should have root1 and root2
    assert!(total >= 2, "Should have at least 2 root nodes");
    assert!(
        nodes.iter().any(|n| n.name == root1_key),
        "Should include root1"
    );
    assert!(
        nodes.iter().any(|n| n.name == root2_key),
        "Should include root2"
    );

    // Browse root1 path
    let (child_nodes, _) = pub_repo.browse_by_path(&root1_path, 100, 0).await?;
    assert!(
        child_nodes.iter().any(|n| n.name == child1_key),
        "Should include child1 under root1 path"
    );

    Ok(())
}

/// Test browsing with pagination
#[tokio::test]
async fn test_browse_by_path_pagination() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_pagination");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    // Create multiple entities
    let repo = DynamicEntityRepository::new(pool.clone());
    for i in 0..10 {
        let entity = create_test_dynamic_entity(
            &entity_def,
            &format!("Entity {}", i),
            "/",
            &Uuid::now_v7().to_string(),
        );
        repo.create(&entity).await?;
    }

    // Browse with limit
    let (page1, total) = pub_repo.browse_by_path("/", 5, 0).await?;
    assert_eq!(page1.len(), 5, "First page should have 5 items");
    assert_eq!(total, 10, "Total should be 10");

    // Browse second page
    let (page2, _) = pub_repo.browse_by_path("/", 5, 5).await?;
    assert_eq!(page2.len(), 5, "Second page should have 5 items");

    Ok(())
}
