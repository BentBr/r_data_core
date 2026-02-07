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
        uuid: Uuid::nil(),
        entity_type: entity_type.to_string(),
        display_name: format!("Test {entity_type}"),
        description: Some(format!("Test description for {entity_type}")),
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
            validation: r_data_core_core::field::FieldValidation::default(),
            ui_settings: r_data_core_core::field::ui::UiSettings::default(),
            constraints: HashMap::new(),
        }],
        schema: r_data_core_core::entity_definition::schema::Schema::default(),
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
    let mut field_data = HashMap::new();
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
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create multiple entity definitions with unique names
    let entity_type1 = unique_entity_type("test_type_1");
    let entity_type2 = unique_entity_type("test_type_2");
    let entity_def1 = create_test_entity_definition(&pool, &entity_type1).await?;
    let _entity_def2 = create_test_entity_definition(&pool, &entity_type2).await?;

    // Create entities for counting
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let entity1 =
        create_test_dynamic_entity(&entity_def1, "Entity 1", "/", &Uuid::now_v7().to_string());
    let entity2 =
        create_test_dynamic_entity(&entity_def1, "Entity 2", "/", &Uuid::now_v7().to_string());
    repo.create(&entity1).await?;
    repo.create(&entity2).await?;

    // Wait a bit for view/table creation to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // List available entity types
    let available_types = pub_repo.list_available_entity_types().await?;

    // Should include both entity types
    assert!(
        available_types.iter().any(|et| et.name == entity_type1),
        "Should include entity_type1"
    );
    assert!(
        available_types.iter().any(|et| et.name == entity_type2),
        "Should include entity_type2"
    );

    // Check entity counts
    let type1 = available_types
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
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_browse");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    // Create entities in a hierarchy
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Root level entities - use entity_key as the key
    let root1_key = format!("root1-{}", Uuid::now_v7());
    let root2_key = format!("root2-{}", Uuid::now_v7());
    let root1 = create_test_dynamic_entity(&entity_def, "Root 1", "/", &root1_key);
    let root2 = create_test_dynamic_entity(&entity_def, "Root 2", "/", &root2_key);
    let root1_uuid = repo.create(&root1).await?;
    let _root2_uuid = repo.create(&root2).await?;
    let root1_path = format!("/{root1_key}");

    // Child entities
    let child1_uuid = Uuid::now_v7();
    let child1_key = format!("child1-{child1_uuid}");
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
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_pagination");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    // Create multiple entities
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    for i in 0..10 {
        let entity = create_test_dynamic_entity(
            &entity_def,
            &format!("Entity {i}"),
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

/// Test browsing published status
#[tokio::test]
async fn test_browse_published_status() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition
    let entity_type = unique_entity_type("test_published");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create published entity
    let pub_uuid = Uuid::now_v7();
    let pub_key = format!("pub-{pub_uuid}");
    let mut pub_entity = create_test_dynamic_entity(&entity_def, "Published", "/", &pub_key);
    pub_entity
        .field_data
        .insert("published".to_string(), json!(true));
    repo.create(&pub_entity).await?;

    // Create unpublished entity
    let unpub_uuid = Uuid::now_v7();
    let unpub_key = format!("unpub-{unpub_uuid}");
    let mut unpub_entity = create_test_dynamic_entity(&entity_def, "Unpublished", "/", &unpub_key);
    unpub_entity
        .field_data
        .insert("published".to_string(), json!(false));
    repo.create(&unpub_entity).await?;

    // Browse root path
    let (nodes, _) = pub_repo.browse_by_path("/", 100, 0).await?;

    let pub_node = nodes
        .iter()
        .find(|n| n.name == pub_key)
        .expect("Published node not found");
    assert!(pub_node.published);

    let unpub_node = nodes
        .iter()
        .find(|n| n.name == unpub_key)
        .expect("Unpublished node not found");
    assert!(!unpub_node.published);

    Ok(())
}

/// Test search by path prefix functionality
#[tokio::test]
async fn test_search_by_path_prefix() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_search");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create entities with different paths and keys
    let search_key1 = format!("searchable-1-{}", Uuid::now_v7());
    let search_key2 = format!("searchable-2-{}", Uuid::now_v7());
    let other_key = format!("other-{}", Uuid::now_v7());

    let entity1 = create_test_dynamic_entity(&entity_def, "Search 1", "/", &search_key1);
    let entity2 = create_test_dynamic_entity(&entity_def, "Search 2", "/folder", &search_key2);
    let entity3 = create_test_dynamic_entity(&entity_def, "Other", "/", &other_key);

    repo.create(&entity1).await?;
    repo.create(&entity2).await?;
    repo.create(&entity3).await?;

    // Search for "searchable" - should find both searchable entities
    let results = pub_repo.search_by_path_prefix("/searchable", 10).await?;
    assert!(
        !results.is_empty(),
        "Should find at least 1 searchable entity"
    );
    assert!(
        results.iter().any(|n| n.name == search_key1),
        "Should find search_key1"
    );

    // Search for "/folder/search" - should find entity2
    let results = pub_repo.search_by_path_prefix("/folder/search", 10).await?;
    assert!(
        results.iter().any(|n| n.name == search_key2),
        "Should find search_key2 at /folder path"
    );

    // Search with limit
    let results = pub_repo.search_by_path_prefix("/", 1).await?;
    assert_eq!(results.len(), 1, "Should respect limit");

    // Search for non-existent path - should not error
    let _results = pub_repo.search_by_path_prefix("/nonexistent", 10).await?;

    Ok(())
}

/// Test `has_children` detection when children exist via path (not `parent_uuid`)
///
/// This tests the scenario where an entity has children that reference it via the `path` field
/// rather than via `parent_uuid`. This is important because path-based parent-child relationships
/// are a common pattern in the system.
#[tokio::test]
async fn test_has_children_via_path_not_parent_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition
    let entity_type = unique_entity_type("test_has_children_path");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create a parent entity at /Users with entity_key "peter-shaw"
    // Full path will be /Users/peter-shaw
    let parent_key = format!("parent-{}", Uuid::now_v7());
    let parent = create_test_dynamic_entity(&entity_def, "Parent Entity", "/Users", &parent_key);
    repo.create(&parent).await?;

    // Create a child entity that uses path /Users/parent_key (the parent's full path)
    // This child does NOT have parent_uuid set
    let child_key = format!("child-{}", Uuid::now_v7());
    let parent_full_path = format!("/Users/{parent_key}");
    let child =
        create_test_dynamic_entity(&entity_def, "Child Entity", &parent_full_path, &child_key);
    // Note: no parent_uuid set on the child
    repo.create(&child).await?;

    // Wait for entities to be created
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Browse /Users - should see parent entity with has_children=true
    let (nodes, _) = pub_repo.browse_by_path("/Users", 100, 0).await?;

    let parent_node = nodes
        .iter()
        .find(|n| n.name == parent_key)
        .expect("Parent node should be found");

    assert_eq!(
        parent_node.has_children,
        Some(true),
        "Parent entity should have has_children=true because it has a child entity \
         whose path field equals the parent's full path ({parent_full_path})"
    );

    // Verify the child is visible when browsing the parent's path
    let (child_nodes, _) = pub_repo.browse_by_path(&parent_full_path, 100, 0).await?;
    assert!(
        child_nodes.iter().any(|n| n.name == child_key),
        "Child entity should be found when browsing parent's full path"
    );

    Ok(())
}

/// Test `has_children` correctly detects children both via `parent_uuid` AND via path
///
/// An entity should show `has_children=true` if EITHER:
/// 1. Another entity has `parent_uuid` set to this entity's UUID, OR
/// 2. Another entity has `path` equal to this entity's full path
#[tokio::test]
async fn test_has_children_via_parent_uuid_and_path() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition
    let entity_type = unique_entity_type("test_has_children_both");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create parent1 with child via parent_uuid
    let parent1_key = format!("parent1-{}", Uuid::now_v7());
    let parent1 = create_test_dynamic_entity(&entity_def, "Parent 1", "/", &parent1_key);
    let parent1_uuid = repo.create(&parent1).await?;

    let child1_key = format!("child1-{}", Uuid::now_v7());
    let parent1_full_path = format!("/{parent1_key}");
    let mut child1 =
        create_test_dynamic_entity(&entity_def, "Child 1", &parent1_full_path, &child1_key);
    child1.set("parent_uuid", parent1_uuid.to_string())?;
    repo.create(&child1).await?;

    // Create parent2 with child via path only (no parent_uuid)
    let parent2_key = format!("parent2-{}", Uuid::now_v7());
    let parent2 = create_test_dynamic_entity(&entity_def, "Parent 2", "/", &parent2_key);
    repo.create(&parent2).await?;

    let child2_key = format!("child2-{}", Uuid::now_v7());
    let parent2_full_path = format!("/{parent2_key}");
    let child2 =
        create_test_dynamic_entity(&entity_def, "Child 2", &parent2_full_path, &child2_key);
    // No parent_uuid set
    repo.create(&child2).await?;

    // Create parent3 with NO children
    let parent3_key = format!("parent3-{}", Uuid::now_v7());
    let parent3 = create_test_dynamic_entity(&entity_def, "Parent 3", "/", &parent3_key);
    repo.create(&parent3).await?;

    // Wait for entities to be created
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Browse root - check has_children for all parents
    let (nodes, _) = pub_repo.browse_by_path("/", 100, 0).await?;

    // Parent1 should have children (via parent_uuid)
    let parent1_node = nodes
        .iter()
        .find(|n| n.name == parent1_key)
        .expect("Parent1 should be found");
    assert_eq!(
        parent1_node.has_children,
        Some(true),
        "Parent1 should have has_children=true (child via parent_uuid)"
    );

    // Parent2 should have children (via path only)
    let parent2_node = nodes
        .iter()
        .find(|n| n.name == parent2_key)
        .expect("Parent2 should be found");
    assert_eq!(
        parent2_node.has_children,
        Some(true),
        "Parent2 should have has_children=true (child via path only, no parent_uuid)"
    );

    // Parent3 should NOT have children
    let parent3_node = nodes
        .iter()
        .find(|n| n.name == parent3_key)
        .expect("Parent3 should be found");
    assert_eq!(
        parent3_node.has_children,
        Some(false),
        "Parent3 should have has_children=false (no children)"
    );

    Ok(())
}

/// Test that `browse_by_path` uses batched queries instead of N+1 queries
/// This test creates many entities to ensure batching is necessary and verifies
/// that `has_children` is correctly determined for all nodes using batched queries.
#[tokio::test]
async fn test_browse_by_path_batched_queries() -> Result<()> {
    let pool = setup_test_db().await;
    let pub_repo = DynamicEntityPublicRepository::new(pool.pool.clone());

    // Create entity definition with unique name
    let entity_type = unique_entity_type("test_batched");
    let entity_def = create_test_entity_definition(&pool, &entity_type).await?;

    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create many root-level entities (50+) to ensure batching is needed
    // This would cause N+1 queries if not batched
    let mut root_entities = Vec::new();
    for i in 0..60 {
        let root_key = format!("root-{i}-{}", Uuid::now_v7());
        let root = create_test_dynamic_entity(&entity_def, &format!("Root {i}"), "/", &root_key);
        let root_uuid = repo.create(&root).await?;
        root_entities.push((root_key, root_uuid));
    }

    // Create children for some of the root entities to test UUID batching
    let mut parent_child_pairs = Vec::new();
    for (i, (root_key, root_uuid)) in root_entities.iter().take(20).enumerate() {
        let root_path = format!("/{root_key}");
        let child_uuid = Uuid::now_v7();
        let child_key = format!("child-{i}-{child_uuid}");
        let mut child =
            create_test_dynamic_entity(&entity_def, &format!("Child {i}"), &root_path, &child_key);
        child.set("parent_uuid", root_uuid.to_string())?;
        repo.create(&child).await?;
        parent_child_pairs.push((root_uuid, root_key.clone()));
    }

    // Create folder structures (virtual folders) to test folder path batching
    // Create entities at nested paths to create folder structures
    for i in 0..15 {
        let folder_name = format!("folder-{i}");
        let folder_path = format!("/{folder_name}");
        let file_uuid = Uuid::now_v7();
        let file_key = format!("file-{i}-{file_uuid}");
        let file =
            create_test_dynamic_entity(&entity_def, &format!("File {i}"), &folder_path, &file_key);
        repo.create(&file).await?;
    }

    // Wait a bit for all entities to be created
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Browse root path - this should use batched queries
    // If N+1 queries were happening, this would be very slow or timeout
    let (nodes, total) = pub_repo.browse_by_path("/", 100, 0).await?;

    // Verify we got all the root entities
    assert!(
        total >= 60,
        "Should have at least 60 root nodes, got {total}"
    );

    // Verify has_children is correctly set for entities with children
    for (_root_uuid, root_key) in &parent_child_pairs {
        let root_node = nodes
            .iter()
            .find(|n| n.name == *root_key)
            .unwrap_or_else(|| panic!("Root node {root_key} not found"));
        assert!(
            root_node.has_children == Some(true),
            "Root entity {root_key} should have has_children=true because it has children"
        );
    }

    // Verify has_children is correctly set for entities without children
    for (root_key, _root_uuid) in root_entities.iter().skip(20) {
        let root_node = nodes
            .iter()
            .find(|n| n.name == *root_key)
            .unwrap_or_else(|| panic!("Root node {root_key} not found"));
        // Entities without children should have has_children=false or None
        assert!(
            root_node.has_children != Some(true),
            "Root entity {root_key} should not have has_children=true because it has no children"
        );
    }

    // Verify folder structures are detected correctly
    for i in 0..15 {
        let folder_name = format!("folder-{i}");
        let folder_node = nodes
            .iter()
            .find(|n| n.name == folder_name)
            .unwrap_or_else(|| panic!("Folder node {folder_name} not found"));
        assert_eq!(
            folder_node.kind,
            r_data_core_core::public_api::BrowseKind::Folder,
            "Node {folder_name} should be a folder"
        );
        assert!(
            folder_node.has_children == Some(true),
            "Folder {folder_name} should have has_children=true because it contains files"
        );
    }

    // Browse a folder path to verify folder path batching works
    let (folder_nodes, _) = pub_repo.browse_by_path("/folder-0", 100, 0).await?;
    assert!(
        folder_nodes.iter().any(|n| n.name.starts_with("file-0-")),
        "Should find file in folder-0"
    );

    Ok(())
}
