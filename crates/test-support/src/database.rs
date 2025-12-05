#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{debug, info};
use r_data_core_core::error::Result;
use sqlx::{postgres::PgPoolOptions, postgres::PgRow, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// Generate a unique entity type name to avoid conflicts between tests
/// Uses UUID to ensure uniqueness across parallel tests
#[must_use]
pub fn unique_entity_type(base: &str) -> String {
    let uuid = Uuid::now_v7();
    format!("{base}_{}", uuid.simple())
}

/// Generate a random string for testing
#[must_use]
pub fn random_string(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::now_v7())
}

/// Generate a unique schema name for a test
/// Uses UUID to ensure uniqueness across parallel tests
#[must_use]
fn generate_test_schema_name() -> String {
    let uuid = Uuid::now_v7();
    // Use a shorter format to avoid PostgreSQL identifier length limits
    format!("test_{}", uuid.simple())
}

/// Run migrations in the test schema
///
/// # Errors
/// Returns an error if migration fails
async fn setup_test_schema(pool: &PgPool, schema_name: &str) -> Result<()> {
    // Ensure search_path is set before running migrations
    // This is important because migrations need to run in the test schema
    let mut conn = pool.acquire().await?;
    sqlx::query(&format!("SET search_path TO \"{schema_name}\", public"))
        .execute(&mut *conn)
        .await?;
    drop(conn);

    // Run migrations in the test schema context
    debug!("Running migrations in schema: {schema_name}");

    match sqlx::migrate!("../../migrations").run(pool).await {
        Ok(()) => {
            debug!("Migrations completed successfully in schema: {schema_name}");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("already exists") {
                debug!("Some migration objects already exist in schema {schema_name}, continuing");
                Ok(())
            } else {
                // Convert MigrateError to our Error type
                Err(r_data_core_core::error::Error::Database(
                    sqlx::Error::Configuration(
                        format!("Failed to run migrations in schema {schema_name}: {e}").into(),
                    ),
                ))
            }
        }
    }
}

/// Set up a test database connection with per-test schema isolation
///
/// Each test gets its own `PostgreSQL` schema, allowing parallel execution
/// without conflicts. The schema is automatically created and migrations
/// are run in the test-specific schema.
///
/// # Panics
/// Panics if `DATABASE_URL` is not set in `.env.test` or if database connection fails
#[must_use]
pub async fn setup_test_db() -> PgPool {
    // Load environment variables from .env.test
    dotenvy::from_filename(".env.test").ok();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set in .env.test");

    // Generate a unique schema name for this test
    let schema_name = generate_test_schema_name();
    debug!("Setting up test database with schema: {schema_name}");

    // Create a temporary connection to create the schema first
    let temp_pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Create the schema first
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""))
        .execute(&temp_pool)
        .await
        .expect("Failed to create test schema");

    // Now create the main pool with after_connect hook to set search_path
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .after_connect({
            let schema_name = schema_name.clone();
            move |conn, _meta| {
                let schema_name = schema_name.clone();
                Box::pin(async move {
                    // Set search_path on each new connection to use the test schema
                    sqlx::query(&format!("SET search_path TO \"{schema_name}\", public"))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            }
        })
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations in the test schema
    if let Err(e) = setup_test_schema(&pool, &schema_name).await {
        panic!("Failed to set up test schema {schema_name}: {e}");
    }

    debug!("Test database setup complete with schema: {schema_name}");

    // Return the dedicated pool for this test
    pool
}

/// Clear all data from the database - optimized version for faster test runs
///
/// # Errors
/// Returns an error if database operations fail
pub async fn fast_clear_test_db(pool: &PgPool) -> Result<()> {
    debug!("Fast clearing test database data");

    // Use a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Disable foreign key constraints during cleanup
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get the main entity tables
    // Clear these key tables but NOT entity_definitions to avoid race conditions
    let mut tables = vec![
        "entities_registry".to_string(),
        "admin_users".to_string(),
        "api_keys".to_string(),
        "refresh_tokens".to_string(),
    ];

    // Also find all entity_* tables in the current schema
    // Use current_schema() to get the schema from search_path
    let entity_tables: Vec<String> = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = current_schema()
         AND tablename LIKE 'entity_%'",
    )
    .map(|row: PgRow| row.get::<String, _>(0))
    .fetch_all(&mut *tx)
    .await?;

    tables.extend(entity_tables);

    // Truncate all specified tables in a single statement
    if !tables.is_empty() {
        let tables_sql = tables
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let truncate_sql = format!("TRUNCATE TABLE {tables_sql} CASCADE");
        debug!("Truncating tables: {truncate_sql}");
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
///
/// # Errors
/// Returns an error if database operations fail
pub async fn clear_entity_definitions(pool: &PgPool) -> Result<()> {
    debug!("Clearing entity definitions");

    sqlx::query("TRUNCATE TABLE entity_definitions CASCADE")
        .execute(pool)
        .await?;

    debug!("Class definitions cleared successfully");
    Ok(())
}

/// Clear all data from the database - original thorough version
///
/// # Errors
/// Returns an error if database operations fail
pub async fn clear_test_db(pool: &PgPool) -> Result<()> {
    info!("Clearing test database data");

    // Use a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Disable foreign key constraints during cleanup
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get all tables except migration table in the current schema
    let tables: Vec<String> = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = current_schema()
         AND tablename != 'schema_migrations'
         AND tablename != '_sqlx_migrations'",
    )
    .map(|row: PgRow| row.get::<String, _>(0))
    .fetch_all(&mut *tx)
    .await?;

    // Truncate all tables in a single statement
    if !tables.is_empty() {
        let tables_sql = tables
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let truncate_sql = format!("TRUNCATE TABLE {tables_sql} CASCADE");
        info!("Truncating tables: {truncate_sql}");
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
///
/// # Errors
/// Returns an error if database operations fail
pub async fn clear_refresh_tokens(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM refresh_tokens")
        .execute(pool)
        .await?;
    Ok(())
}
