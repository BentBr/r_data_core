#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::{DynamicEntityRepository, EntityDefinitionRepository};
use r_data_core_services::EntityDefinitionService;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::database::GLOBAL_TEST_MUTEX;

/// Create a test entity definition
///
/// # Errors
/// Returns an error if entity definition creation fails
#[allow(clippy::await_holding_lock, clippy::future_not_send)] // MutexGuard is intentionally held across await for test isolation
pub async fn create_test_entity_definition(pool: &PgPool, entity_type: &str) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Create a simple entity definition for testing
    let mut entity_def = EntityDefinition {
        entity_type: entity_type.to_string(),
        display_name: format!("{entity_type} Class"),
        description: Some(format!("Test description for {entity_type}")),
        published: true,
        ..EntityDefinition::default()
    };

    // Add fields to the entity definition
    let mut fields = Vec::new();

    // Name field
    let name_field = FieldDefinition {
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
    };
    fields.push(name_field);

    // Email field
    let email_field = FieldDefinition {
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
    };
    fields.push(email_field);

    entity_def.fields = fields;

    // Set created_by
    let created_by = Uuid::now_v7();
    entity_def.created_by = created_by;

    // Use the repository trait to create the entity definition
    let repository = EntityDefinitionRepository::new(pool.clone());
    let service = EntityDefinitionService::new_without_cache(Arc::new(repository));

    // Create the entity definition and wait for the service to finish
    let uuid = service.create_entity_definition(&entity_def).await?;

    // Wait a moment for the view creation (the service should trigger this)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(uuid)
}

/// Create a test entity
///
/// # Panics
/// May panic if database operations fail
///
/// # Errors
/// Returns an error if entity creation fails
#[allow(clippy::await_holding_lock, clippy::future_not_send)] // MutexGuard is intentionally held across await for test isolation
pub async fn create_test_entity(
    pool: &PgPool,
    entity_type: &str,
    name: &str,
    email: &str,
) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("name".to_string(), json!(name));
    field_data.insert("email".to_string(), json!(email));
    // Provide required registry fields for tests
    field_data.insert(
        "entity_key".to_string(),
        json!(format!(
            "{}-{}",
            name.to_lowercase().replace(' ', "-"),
            uuid.simple()
        )),
    );
    field_data.insert("path".to_string(), json!("/"));
    field_data.insert("created_by".to_string(), json!(created_by.to_string()));
    field_data.insert(
        "created_at".to_string(),
        json!(OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()),
    );
    field_data.insert("version".to_string(), json!(1));
    field_data.insert("published".to_string(), json!(true));

    // First get the entity definition for this entity type
    let class_repo = EntityDefinitionRepository::new(pool.clone());
    let class_service = EntityDefinitionService::new_without_cache(Arc::new(class_repo));
    let entity_def = class_service
        .get_entity_definition_by_entity_type(entity_type)
        .await?;

    let entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: Arc::new(entity_def),
    };

    // Create the entity
    let repository = DynamicEntityRepository::new(pool.clone());
    repository.create(&entity).await?;

    // Allow time for any triggers to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(uuid)
}

/// Create a test admin user with a guaranteed unique username
///
/// # Errors
/// Returns an error if database operations fail
#[allow(clippy::await_holding_lock, clippy::future_not_send)] // MutexGuard is intentionally held across await for test isolation
pub async fn create_test_admin_user(pool: &PgPool) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Generate a truly unique username with UUID
    let uuid = Uuid::now_v7();
    let username = format!("test_admin_{}", uuid.simple());
    let email = format!("test_{}@example.com", uuid.simple());

    // Use a transaction to ensure atomicity
    let mut tx = pool.begin().await?;

    // First check if user already exists (unlikely with UUID but ensures idempotence)
    let count: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM admin_users WHERE username = $1 OR email = $2",
    )
    .bind(&username)
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;

    let exists = count > 0;

    if exists {
        log::debug!("User already exists, returning UUID: {uuid}");
        tx.commit().await?;
        return Ok(uuid);
    }

    // Create a new admin user with super_admin = true for tests
    log::debug!("Creating test admin user: {username}");
    sqlx::query("INSERT INTO admin_users (uuid, username, email, password_hash, first_name, last_name, is_active, created_at, updated_at, created_by, super_admin)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $1, $9)")
        .bind(uuid)
        .bind(&username)
        .bind(&email)
        .bind("$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4")  // Hash of "adminadmin"
        .bind("Test")
        .bind("User")
        .bind(true)
        .bind(OffsetDateTime::now_utc())
        .bind(true) // super_admin = true for test users
        .execute(&mut *tx)
        .await?;

    // Commit the transaction
    log::debug!("Committing new admin user transaction");
    tx.commit().await?;

    log::debug!("Created test admin user with UUID: {uuid}");
    Ok(uuid)
}

/// Create a test API key
///
/// # Errors
/// Returns an error if database operations fail
#[allow(clippy::await_holding_lock, clippy::future_not_send)] // MutexGuard is intentionally held across await for test isolation
pub async fn create_test_api_key(pool: &PgPool, api_key: String) -> Result<()> {
    use r_data_core_core::admin_user::ApiKey;

    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Create an admin user first with unique values
    let admin_uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();
    let unique_id = Uuid::now_v7().simple();
    let username = format!("test_admin_{unique_id}");
    let email = format!("admin_{unique_id}@example.test");

    sqlx::query(
        "INSERT INTO admin_users (uuid, path, username, email, password_hash, created_at, created_by, published)
         VALUES ($1, '/users', $2, $3, $4, NOW(), $5, true)"
    )
        .bind(admin_uuid)
        .bind(username)
        .bind(email)
        .bind("$2a$12$Uei2P1KLrTSn9.XqtBBSHelmkRgJpYx2FkqKrOurOt8DG.PJiElFy") // hashed "password"
        .bind(created_by)
        .execute(pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    // Create an API key
    let key_uuid = Uuid::now_v7();

    // Hash the API key properly
    let key_hash = ApiKey::hash_api_key(&api_key)
        .map_err(|e| r_data_core_core::error::Error::Unknown(e.to_string()))?;

    sqlx::query(
        "INSERT INTO api_keys (uuid, user_uuid, name, key_hash, is_active, created_at, created_by, published)
         VALUES ($1, $2, $3, $4, true, NOW(), $5, true)"
    )
        .bind(key_uuid)
        .bind(admin_uuid)
        .bind("Test API Key")
        .bind(key_hash)  // Use the properly hashed key
        .bind(created_by)
        .execute(pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    Ok(())
}

/// Get the username for the test admin user
///
/// # Panics
/// May panic if UUID generation fails
#[must_use]
pub fn get_test_user_username() -> String {
    // Generate a consistent username for tests
    // This should match the pattern used in create_test_admin_user
    let uuid = Uuid::now_v7();
    format!("test_admin_{}", uuid.simple())
}

/// Create an entity definition from a JSON file
///
/// # Panics
/// May panic if database operations fail
///
/// # Errors
/// Returns an error if file reading, parsing, or entity creation fails
#[allow(clippy::await_holding_lock, clippy::future_not_send)] // MutexGuard is intentionally held across await for test isolation
pub async fn create_entity_definition_from_json(pool: &PgPool, json_path: &str) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Read the JSON file
    let json_content = std::fs::read_to_string(json_path).map_err(|e| {
        r_data_core_core::error::Error::Unknown(format!(
            "Failed to read JSON file {json_path}: {e}"
        ))
    })?;

    // Parse the JSON into a EntityDefinition
    let mut entity_def: EntityDefinition = serde_json::from_str(&json_content).map_err(|e| {
        r_data_core_core::error::Error::Unknown(format!(
            "Failed to parse JSON file {json_path}: {e}"
        ))
    })?;

    // Make the entity type unique to avoid test conflicts
    let unique_entity_type = crate::database::unique_entity_type(&entity_def.entity_type);
    entity_def.entity_type = unique_entity_type;

    // Make sure we have a creator
    if entity_def.created_by == Uuid::nil() {
        entity_def.created_by = Uuid::now_v7();
    }

    // Set created_at and updated_at if not present
    if entity_def.created_at == OffsetDateTime::UNIX_EPOCH {
        entity_def.created_at = OffsetDateTime::now_utc();
    }
    if entity_def.updated_at == OffsetDateTime::UNIX_EPOCH {
        entity_def.updated_at = OffsetDateTime::now_utc();
    }

    // Create the entity definition using the service
    let repository = EntityDefinitionRepository::new(pool.clone());
    let service = EntityDefinitionService::new_without_cache(Arc::new(repository));
    let uuid = service.create_entity_definition(&entity_def).await?;

    // The view creation should be handled by the service, but we'll add a small delay
    // to ensure all database operations complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    Ok(uuid)
}
