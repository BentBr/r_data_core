use actix_web;
use dotenv;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::entity::dynamic_entity::entity::DynamicEntity;
use r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository;
use r_data_core::entity::entity_definition::definition::EntityDefinition;
use r_data_core::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core::entity::field::ui::UiSettings;
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::error::{Error, Result};
use r_data_core::services::{DynamicEntityService, EntityDefinitionService};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

// Global constants and state for test synchronization
lazy_static! {
    // Global mutex for DB operations
    pub static ref GLOBAL_TEST_MUTEX: Mutex<()> = Mutex::new(());

    // Track the last used entity type to ensure uniqueness
    static ref ENTITY_TYPE_COUNTER: Mutex<u32> = Mutex::new(0);

    // New: keep track of DB initialization to avoid duplicate setups
    static ref DB_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
}

/// Generate a unique entity type name to avoid conflicts between tests
#[allow(dead_code)]
pub fn unique_entity_type(base: &str) -> String {
    let mut counter = ENTITY_TYPE_COUNTER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let count = *counter;
    *counter = count + 1;
    format!("{}_{}", base, count)
}

/// Generate a random string for testing
#[allow(dead_code)]
pub fn random_string(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::now_v7())
}

/// Create a test entity definition
#[allow(dead_code)]
pub async fn create_test_entity_definition(pool: &PgPool, entity_type: &str) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    // Create a simple entity definition for testing
    let mut entity_def = EntityDefinition::default();
    entity_def.entity_type = entity_type.to_string();
    entity_def.display_name = format!("{} Class", entity_type);
    entity_def.description = Some(format!("Test description for {}", entity_type));
    entity_def.published = true;

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
    let service = EntityDefinitionService::new(Arc::new(repository));

    // Create the entity definition and wait for the service to finish
    let uuid = service.create_entity_definition(&entity_def).await?;

    // Wait a moment for the view creation (the service should trigger this)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(uuid)
}

/// Create a test entity
#[allow(dead_code)]
pub async fn create_test_entity(
    pool: &PgPool,
    entity_type: &str,
    name: &str,
    email: &str,
) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("name".to_string(), json!(name));
    field_data.insert("email".to_string(), json!(email));
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
    let class_service = EntityDefinitionService::new(Arc::new(class_repo));
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

/// Create a test API key
#[allow(dead_code)]
pub async fn create_test_api_key(pool: &PgPool, api_key: String) -> Result<()> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    use r_data_core::entity::admin_user::model::ApiKey;

    // Create an admin user first with unique values
    let admin_uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();
    let unique_id = Uuid::now_v7().simple();
    let username = format!("test_admin_{}", unique_id);
    let email = format!("admin_{}@example.test", unique_id);

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
        .map_err(Error::Database)?;

    // Create an API key
    let key_uuid = Uuid::now_v7();

    // Hash the API key properly
    let key_hash = ApiKey::hash_api_key(&api_key).map_err(|e| Error::Unknown(e.to_string()))?;

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
        .map_err(Error::Database)?;

    Ok(())
}

/// Set up a test database connection
#[allow(dead_code)]
pub async fn setup_test_db() -> PgPool {
    // Get global lock for the entire test run
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    info!("Setting up test database with global lock acquired");

    // Load environment variables from .env.test
    dotenv::from_filename(".env.test").ok();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set in .env.test");
    info!("Connecting to test database: {}", database_url);

    // Create a dedicated connection pool for this test - use smaller pool and timeout for tests
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Check if we need to initialize the database
    let db_initialized = DB_READY.load(std::sync::atomic::Ordering::Acquire);

    if !db_initialized {
        info!("First-time database initialization");

        // First clean the database if it exists already
        info!("Dropping existing schema if any");
        let _ = sqlx::query("DROP SCHEMA public CASCADE")
            .execute(&pool)
            .await;
        let _ = sqlx::query("CREATE SCHEMA public").execute(&pool).await;

        // Run migrations - this handles schema creation
        info!("Running database migrations");
        match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => info!("Database migrations completed successfully"),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    info!("Some migration objects already exist, continuing");
                } else {
                    panic!("Failed to run migrations: {}", e);
                }
            }
        }

        // Set the initialization flag to avoid redoing this work
        DB_READY.store(true, std::sync::atomic::Ordering::Release);
    } else {
        // If database is already initialized, just clear the data
        if let Err(e) = fast_clear_test_db(&pool).await {
            warn!("Warning: Failed to clear test database: {}", e);
        }
    }

    // Return the dedicated pool for this test
    pool
}

/// Clear all data from the database - optimized version for faster test runs
#[allow(dead_code)]
pub async fn fast_clear_test_db(pool: &PgPool) -> Result<()> {
    debug!("Fast clearing test database data");

    // Use a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Disable foreign key constraints during cleanup
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get the main entity tables
    let mut tables = Vec::new();

    // Clear these key tables but NOT entity_definitions to avoid race conditions
    tables.push("entities_registry".to_string());
    tables.push("admin_users".to_string());
    tables.push("api_keys".to_string());
    tables.push("refresh_tokens".to_string());

    // Also find all entity_* tables
    let entity_tables = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = 'public'
         AND tablename LIKE 'entity_%'",
    )
    .map(|row: sqlx::postgres::PgRow| row.get::<String, _>(0))
    .fetch_all(&mut *tx)
    .await?;

    tables.extend(entity_tables);

    // Truncate all specified tables in a single statement
    if !tables.is_empty() {
        let tables_sql = tables
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(", ");

        let truncate_sql = format!("TRUNCATE TABLE {} CASCADE", tables_sql);
        debug!("Truncating tables: {}", truncate_sql);
        sqlx::query(&truncate_sql).execute(&mut *tx).await?;
    }

    // Re-enable foreign key constraints
    sqlx::query("SET session_replication_role = 'origin'")
        .execute(&mut *tx)
        .await?;

    // Commit transaction
    tx.commit().await?;

    debug!("Test database cleared successfully");
    Ok(())
}

/// Clear entity definitions separately when needed
#[allow(dead_code)]
pub async fn clear_entity_definitions(pool: &PgPool) -> Result<()> {
    debug!("Clearing entity definitions");

    sqlx::query("TRUNCATE TABLE entity_definitions CASCADE")
        .execute(pool)
        .await?;

    debug!("Class definitions cleared successfully");
    Ok(())
}

/// Clear all data from the database - original thorough version
#[allow(dead_code)]
pub async fn clear_test_db(pool: &PgPool) -> Result<()> {
    info!("Clearing test database data");

    // Use a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Disable foreign key constraints during cleanup
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get all tables except migration table
    let tables = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = 'public'
         AND tablename != 'schema_migrations'
         AND tablename != '_sqlx_migrations'",
    )
    .map(|row: sqlx::postgres::PgRow| row.get::<String, _>(0))
    .fetch_all(&mut *tx)
    .await?;

    // Truncate all tables in a single statement
    if !tables.is_empty() {
        let tables_sql = tables
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(", ");

        let truncate_sql = format!("TRUNCATE TABLE {} CASCADE", tables_sql);
        info!("Truncating tables: {}", truncate_sql);
        sqlx::query(&truncate_sql).execute(&mut *tx).await?;
    }

    // Re-enable foreign key constraints
    sqlx::query("SET session_replication_role = 'origin'")
        .execute(&mut *tx)
        .await?;

    // Commit transaction
    tx.commit().await?;

    info!("Test database cleared successfully");
    Ok(())
}

/// Clear only refresh tokens table
#[allow(dead_code)]
pub async fn clear_refresh_tokens(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM refresh_tokens")
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a test admin user with a guaranteed unique username
#[allow(dead_code)]
pub async fn create_test_admin_user(pool: &PgPool) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

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
        debug!("User already exists, returning UUID: {}", uuid);
        tx.commit().await?;
        return Ok(uuid);
    }

    // Create a new admin user
    debug!("Creating test admin user: {}", username);
    sqlx::query("INSERT INTO admin_users (uuid, username, email, password_hash, first_name, last_name, is_active, created_at, updated_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $1)")
        .bind(uuid)
        .bind(&username)
        .bind(&email)
        .bind("$argon2id$v=19$m=19456,t=2,p=1$AyU4SymrYGzpmYfqDSbugg$AhzMvJ1bOxrv2WQ1ks3PRFXGezp966kjJwkoUdJbFY4")  // Hash of "adminadmin"
        .bind("Test")
        .bind("User")
        .bind(true)
        .bind(OffsetDateTime::now_utc())
        .execute(&mut *tx)
        .await?;

    // Commit the transaction
    debug!("Committing new admin user transaction");
    tx.commit().await?;

    debug!("Created test admin user with UUID: {}", uuid);
    Ok(uuid)
}

/// Get the username for the test admin user
#[allow(dead_code)]
pub fn get_test_user_username() -> String {
    // Generate a consistent username for tests
    // This should match the pattern used in create_test_admin_user
    let uuid = Uuid::now_v7();
    format!("test_admin_{}", uuid.simple())
}

/// Create a entity definition from a JSON file
#[allow(dead_code)]
pub async fn create_entity_definition_from_json(pool: &PgPool, json_path: &str) -> Result<Uuid> {
    // Acquire a lock for database operations
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    // Read the JSON file
    let json_content = std::fs::read_to_string(json_path)
        .map_err(|e| Error::Unknown(format!("Failed to read JSON file {}: {}", json_path, e)))?;

    // Parse the JSON into a EntityDefinition
    let mut entity_def: EntityDefinition = serde_json::from_str(&json_content)
        .map_err(|e| Error::Unknown(format!("Failed to parse JSON file {}: {}", json_path, e)))?;

    // Make the entity type unique to avoid test conflicts
    let unique_entity_type = unique_entity_type(&entity_def.entity_type);
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
    let service = EntityDefinitionService::new(Arc::new(repository));
    let uuid = service.create_entity_definition(&entity_def).await?;

    // The view creation should be handled by the service, but we'll add a small delay
    // to ensure all database operations complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(uuid)
}

/// Clean up test resources - call this at the end of tests
#[allow(dead_code)]
pub async fn cleanup_test_resources() -> Result<()> {
    // Final cleanup if needed
    // No shared pool to close anymore - each test has its own pool
    Ok(())
}
