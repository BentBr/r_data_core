#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use r_data_core_core::{
    entity_definition::definition::EntityDefinition, entity_definition::schema::Schema,
    field::definition::FieldDefinition, field::types::FieldType,
};
use r_data_core_persistence::{
    DynamicEntityRepository, DynamicEntityRepositoryTrait, FilterEntitiesParams,
};
use r_data_core_test_support::setup_test_db;

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
                unique: false,
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
                unique: false,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: Schema::default(),
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
    let mut field_data = HashMap::new();
    field_data.insert("name".to_string(), json!("John Doe"));
    field_data.insert("age".to_string(), json!(30));
    field_data.insert("entity_key".to_string(), json!(Uuid::now_v7().to_string()));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    DynamicEntity {
        entity_type: entity_definition.entity_type.clone(),
        field_data,
        definition: Arc::new(entity_definition.clone()),
    }
}

// Test for CRUD operations on dynamic entities
#[tokio::test]
async fn test_dynamic_entity_crud() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    // In a real test with a database:
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    // Create a test entity definition
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_def.entity_type)
        .await?;

    // Create a test entity
    let entity = create_test_dynamic_entity(&created_def);

    let entity_uuid = repo.create(&entity).await?;

    assert!(!entity_uuid.is_nil(), "UUID should be valid");

    // Test get by type and UUID - use the returned UUID
    let retrieved = repo
        .get_by_type(&entity.entity_type, &entity_uuid, None)
        .await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.entity_type, entity.entity_type);

    // Test update - need to set UUID in field_data for update to work
    let mut updated_entity = entity.clone();
    updated_entity.set("uuid", entity_uuid.to_string())?;
    updated_entity.set("name", "Jane Doe".to_string())?;
    repo.update(&updated_entity).await?;

    // Verify update
    let retrieved = repo
        .get_by_type(&entity.entity_type, &entity_uuid, None)
        .await?
        .unwrap();
    assert_eq!(retrieved.get::<String>("name")?, "Jane Doe");

    // Test delete
    repo.delete_by_type(&entity.entity_type, &entity_uuid)
        .await?;

    // Verify delete
    let retrieved = repo
        .get_by_type(&entity.entity_type, &entity_uuid, None)
        .await?;
    assert!(retrieved.is_none());

    Ok(())
}

// Test for listing entities of a specific type
#[tokio::test]
async fn test_list_entities_by_type() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    // In a real test with a database:
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    // Create a test entity definition
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_def.entity_type)
        .await?;

    // Create multiple test entities
    let entity1 = create_test_dynamic_entity(&created_def);
    let entity2 = create_test_dynamic_entity(&created_def);

    // Create the entities
    repo.create(&entity1).await?;
    repo.create(&entity2).await?;

    // Test list by type
    let entities = repo
        .get_all_by_type(&entity_def.entity_type, 100, 0, None)
        .await?;
    assert_eq!(entities.len(), 2);

    Ok(())
}

// Test for retrieving entities with a specific parent
#[tokio::test]
async fn test_list_entities_by_parent() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    // In a real test with a database:
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    // Create a test entity definition
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_def.entity_type)
        .await?;

    let parent = create_test_dynamic_entity(&created_def);
    let parent_uuid = repo.create(&parent).await?;

    // Create child entities and set parent reference in field data
    let mut child1 = create_test_dynamic_entity(&created_def);
    child1.set("parent_uuid", parent_uuid.to_string())?;

    let mut child2 = create_test_dynamic_entity(&created_def);
    child2.set("parent_uuid", parent_uuid.to_string())?;

    repo.create(&child1).await?;
    repo.create(&child2).await?;

    // Test filter by parent - using filter_entities method
    let filters = HashMap::from([("parent_uuid".to_string(), json!(parent_uuid.to_string()))]);
    let params = FilterEntitiesParams::new(10, 0).with_filters(Some(filters));
    let children = repo
        .filter_entities(&entity_def.entity_type, &params)
        .await?;
    assert_eq!(children.len(), 2);

    Ok(())
}

// Test for filtering entities based on field values
#[tokio::test]
async fn test_filter_entities() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    // In a real test with a database:
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    // Create a test entity definition
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_def.entity_type)
        .await?;

    // Create test entities with different field values
    let mut entity1 = create_test_dynamic_entity(&created_def);
    entity1.set("name", "Alice".to_string())?;
    entity1.set("age", 25)?;

    let mut entity2 = create_test_dynamic_entity(&created_def);
    entity2.set("name", "Bob".to_string())?;
    entity2.set("age", 30)?;

    let mut entity3 = create_test_dynamic_entity(&created_def);
    entity3.set("name", "Charlie".to_string())?;
    entity3.set("age", 35)?;

    repo.create(&entity1).await?;
    repo.create(&entity2).await?;
    repo.create(&entity3).await?;

    // Test filtering with the filter_entities method
    let filters = HashMap::from([("age".to_string(), json!(30))]);
    let params = FilterEntitiesParams::new(10, 0).with_filters(Some(filters));
    let filtered = repo
        .filter_entities(&entity_def.entity_type, &params)
        .await?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].get::<String>("name")?, "Bob");

    Ok(())
}

// Test for counting entities
#[tokio::test]
async fn test_count_entities() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;

    // In a real test with a database:
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    // Create a test entity definition
    let mut entity_def = create_test_entity_definition_struct();
    entity_def.published = true;
    entity_def.created_by = Uuid::now_v7();

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_def.entity_type)
        .await?;

    // Create multiple test entities
    for i in 0..5 {
        let mut entity = create_test_dynamic_entity(&created_def);
        entity.set("name", format!("Test Entity {i}"))?;
        let _uuid = repo.create(&entity).await?;
    }

    // Test count function
    let count = repo.count_entities(&entity_def.entity_type).await?;
    assert_eq!(count, 5);

    Ok(())
}

/// Test that field names with different cases are handled correctly:
/// - Entity definition uses camelCase (firstName, lastName)
/// - Database stores in lowercase (firstname, lastname)
/// - API returns in entity definition case (firstName, lastName)
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_field_name_case_handling() -> Result<()> {
    use r_data_core_persistence::EntityDefinitionRepository;
    use r_data_core_services::EntityDefinitionService;
    use r_data_core_test_support::{setup_test_db, unique_entity_type};
    use sqlx::Row;

    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // Create a unique entity type for this test
    let entity_type = unique_entity_type("Customer");

    // Create entity definition with camelCase field names
    let mut entity_def = EntityDefinition {
        entity_type: entity_type.clone(),
        display_name: format!("{entity_type} Entity"),
        published: true,
        created_by: Uuid::now_v7(),
        ..Default::default()
    };

    // Add fields with camelCase names (like firstName, lastName)
    entity_def.fields = vec![
        FieldDefinition {
            name: "firstName".to_string(), // camelCase
            display_name: "First Name".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("First name".to_string()),
            filterable: true,
            unique: false,
            indexed: false,
            default_value: None,
            validation: r_data_core_core::field::FieldValidation::default(),
            ui_settings: r_data_core_core::field::ui::UiSettings::default(),
            constraints: HashMap::new(),
        },
        FieldDefinition {
            name: "lastName".to_string(), // camelCase
            display_name: "Last Name".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Last name".to_string()),
            filterable: true,
            unique: false,
            indexed: false,
            default_value: None,
            validation: r_data_core_core::field::FieldValidation::default(),
            ui_settings: r_data_core_core::field::ui::UiSettings::default(),
            constraints: HashMap::new(),
        },
        FieldDefinition {
            name: "email".to_string(), // lowercase
            display_name: "Email".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("Email address".to_string()),
            filterable: true,
            unique: false,
            indexed: true,
            default_value: None,
            validation: r_data_core_core::field::FieldValidation::default(),
            ui_settings: r_data_core_core::field::ui::UiSettings::default(),
            constraints: HashMap::new(),
        },
    ];

    // Create the entity definition in the database
    let def_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Wait for view creation
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get the created entity definition (with proper structure)
    let created_def = def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await?;

    // Create an entity with camelCase field names (matching entity definition)
    let created_by = Uuid::now_v7();
    let mut field_data = HashMap::new();
    field_data.insert("firstName".to_string(), json!("John")); // camelCase
    field_data.insert("lastName".to_string(), json!("Doe")); // camelCase
    field_data.insert("email".to_string(), json!("john.doe@example.com"));
    field_data.insert("entity_key".to_string(), json!("customer-1"));
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert("created_by".to_string(), json!(created_by.to_string()));
    field_data.insert("version".to_string(), json!(1));
    field_data.insert("published".to_string(), json!(true));

    let entity = DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: field_data.clone(),
        definition: Arc::new(created_def.clone()),
    };

    // Test 1: Create entity - field names should be converted to lowercase in database
    let entity_uuid = repo.create(&entity).await?;

    // Test 2: Verify database stores columns in lowercase
    let table_name = format!("entity_{}", entity_type.to_lowercase());
    let row = sqlx::query(&format!(
        "SELECT firstname, lastname, email FROM {table_name} WHERE uuid = $1"
    ))
    .bind(entity_uuid)
    .fetch_optional(&pool.pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    assert!(row.is_some(), "Entity should exist in database");
    let row = row.unwrap();
    let db_firstname: Option<String> = row.try_get("firstname").ok();
    let db_lastname: Option<String> = row.try_get("lastname").ok();
    let db_email: Option<String> = row.try_get("email").ok();

    assert_eq!(
        db_firstname,
        Some("John".to_string()),
        "Database should store firstname in lowercase column"
    );
    assert_eq!(
        db_lastname,
        Some("Doe".to_string()),
        "Database should store lastname in lowercase column"
    );
    assert_eq!(db_email, Some("john.doe@example.com".to_string()));

    // Test 3: Read entity back - field names should be in entity definition case (camelCase)
    let retrieved = repo.get_by_type(&entity_type, &entity_uuid, None).await?;

    assert!(retrieved.is_some(), "Entity should be retrievable");
    let retrieved = retrieved.unwrap();

    // Verify field names are in camelCase (entity definition case)
    assert!(
        retrieved.field_data.contains_key("firstName"),
        "Retrieved entity should have 'firstName' (camelCase) not 'firstname'"
    );
    assert!(
        retrieved.field_data.contains_key("lastName"),
        "Retrieved entity should have 'lastName' (camelCase) not 'lastname'"
    );
    assert!(
        !retrieved.field_data.contains_key("firstname"),
        "Retrieved entity should NOT have 'firstname' (lowercase)"
    );
    assert!(
        !retrieved.field_data.contains_key("lastname"),
        "Retrieved entity should NOT have 'lastname' (lowercase)"
    );

    // Verify values are correct
    assert_eq!(
        retrieved
            .field_data
            .get("firstName")
            .and_then(|v| v.as_str()),
        Some("John"),
        "firstName value should match"
    );
    assert_eq!(
        retrieved
            .field_data
            .get("lastName")
            .and_then(|v| v.as_str()),
        Some("Doe"),
        "lastName value should match"
    );
    assert_eq!(
        retrieved.field_data.get("email").and_then(|v| v.as_str()),
        Some("john.doe@example.com"),
        "email value should match"
    );

    // Test 4: Update entity with camelCase field names
    let mut updated_field_data = retrieved.field_data.clone();
    updated_field_data.insert("firstName".to_string(), json!("Jane")); // Still camelCase
    updated_field_data.insert("lastName".to_string(), json!("Smith")); // Still camelCase

    let updated_entity = DynamicEntity {
        entity_type: entity_type.clone(),
        field_data: updated_field_data,
        definition: Arc::new(created_def.clone()),
    };

    repo.update(&updated_entity).await?;

    // Test 5: Verify update worked and field names are still in camelCase
    let updated_retrieved = repo.get_by_type(&entity_type, &entity_uuid, None).await?;

    assert!(updated_retrieved.is_some());
    let updated_retrieved = updated_retrieved.unwrap();

    assert_eq!(
        updated_retrieved
            .field_data
            .get("firstName")
            .and_then(|v| v.as_str()),
        Some("Jane"),
        "Updated firstName should be 'Jane'"
    );
    assert_eq!(
        updated_retrieved
            .field_data
            .get("lastName")
            .and_then(|v| v.as_str()),
        Some("Smith"),
        "Updated lastName should be 'Smith'"
    );

    // Verify database still has lowercase columns
    let updated_row = sqlx::query(&format!(
        "SELECT firstname, lastname FROM {table_name} WHERE uuid = $1"
    ))
    .bind(entity_uuid)
    .fetch_optional(&pool.pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    assert!(updated_row.is_some());
    let updated_row = updated_row.unwrap();
    let db_firstname: Option<String> = updated_row.try_get("firstname").ok();
    let db_lastname: Option<String> = updated_row.try_get("lastname").ok();

    assert_eq!(
        db_firstname,
        Some("Jane".to_string()),
        "Database should have updated firstname"
    );
    assert_eq!(
        db_lastname,
        Some("Smith".to_string()),
        "Database should have updated lastname"
    );

    Ok(())
}
