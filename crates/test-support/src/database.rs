#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{debug, info, warn};
use r_data_core_core::error::Result;
use sqlx::{postgres::PgPoolOptions, postgres::PgRow, PgPool, Row};
use std::ops::Deref;
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

/// Wrapper around `PgPool` that automatically cleans up the test schema on drop
///
/// This struct ensures that test schemas are automatically dropped when tests
/// complete, preventing accumulation of schemas that consume `PostgreSQL` shared memory.
/// The struct implements `Deref` to `PgPool`, so it can be used transparently
/// wherever a `PgPool` is expected.
///
/// For sqlx queries that require an explicit `&PgPool` type, use `&*pool` or access
/// the `pool` field directly.
pub struct TestDatabase {
    /// The underlying `PostgreSQL` connection pool
    ///
    /// This field is public to allow direct access when needed for sqlx queries
    /// that require an explicit `&PgPool` type rather than a deref target.
    pub pool: PgPool,
    schema_name: String,
    database_url: String,
}

impl Deref for TestDatabase {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl AsRef<PgPool> for TestDatabase {
    fn as_ref(&self) -> &PgPool {
        &self.pool
    }
}

impl Clone for TestDatabase {
    fn clone(&self) -> Self {
        // Clone the pool, but use the same schema name and database URL
        // Only the first TestDatabase to be dropped will clean up the schema
        Self {
            pool: self.pool.clone(),
            schema_name: self.schema_name.clone(),
            database_url: self.database_url.clone(),
        }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let schema_name = self.schema_name.clone();
        let database_url = self.database_url.clone();

        // Spawn a background thread to clean up so we never block an active runtime.
        // Add a small delay to allow any in-flight queries to finish.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));

            if let Ok(rt) = tokio::runtime::Runtime::new() {
                let result = rt.block_on(async {
                    tokio::time::timeout(
                        Duration::from_secs(10),
                        teardown_test_schema_internal(&database_url, &schema_name),
                    )
                    .await
                });

                match result {
                    Ok(Ok(())) => {
                        debug!("Successfully dropped test schema: {schema_name}");
                    }
                    Ok(Err(e)) => {
                        warn!(
                            "Failed to drop test schema {schema_name}: {e}. This may cause shared memory issues."
                        );
                    }
                    Err(_) => {
                        warn!(
                            "Timeout dropping test schema {schema_name}. Schema may remain in database."
                        );
                    }
                }
            } else {
                warn!(
                    "Could not create runtime to drop test schema {schema_name}. Schema may remain in database."
                );
            }
        });
    }
}

/// Internal function to drop a test schema
///
/// # Errors
/// Returns an error if the database connection or drop operation fails
async fn teardown_test_schema_internal(database_url: &str, schema_name: &str) -> Result<()> {
    // Create a temporary connection pool to drop the schema
    // We can't use the pool that's configured for this schema
    let temp_pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Database(sqlx::Error::Configuration(
                format!("Failed to connect to database for schema cleanup: {e}").into(),
            ))
        })?;

    // Drop the schema with CASCADE to handle all dependencies
    sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE"))
        .execute(&temp_pool)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Database(sqlx::Error::Configuration(
                format!("Failed to drop test schema {schema_name}: {e}").into(),
            ))
        })?;

    Ok(())
}

/// Manually drop a test schema
///
/// This function can be used to explicitly drop a schema if needed.
/// Normally, schemas are automatically dropped when `TestDatabase` goes out of scope.
///
/// # Panics
/// Panics if `DATABASE_URL` is not set in `.env.test`
///
/// # Errors
/// Returns an error if the database connection or drop operation fails
pub async fn teardown_test_schema(schema_name: &str) -> Result<()> {
    dotenvy::from_filename(".env.test").ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set in .env.test");

    teardown_test_schema_internal(&database_url, schema_name).await
}

/// Clean up all orphaned test schemas matching the test schema pattern
///
/// This utility function can be used to clean up test schemas that may have been
/// left behind due to test failures or manual runs. It finds all schemas matching
/// the pattern `test_*` and drops them.
///
/// # Panics
/// Panics if `DATABASE_URL` is not set in `.env.test`
///
/// # Errors
/// Returns an error if the database connection or cleanup operation fails
pub async fn cleanup_orphaned_test_schemas() -> Result<usize> {
    dotenvy::from_filename(".env.test").ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set in .env.test");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::Database(sqlx::Error::Configuration(
                format!("Failed to connect to database for cleanup: {e}").into(),
            ))
        })?;

    // Find all test schemas
    let schemas: Vec<String> = sqlx::query(
        "SELECT schema_name FROM information_schema.schemata
         WHERE schema_name LIKE 'test_%'
         AND schema_name != 'test'",
    )
    .map(|row: PgRow| row.get::<String, _>(0))
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Database(sqlx::Error::Configuration(
            format!("Failed to query test schemas: {e}").into(),
        ))
    })?;

    let mut dropped_count = 0;
    for schema_name in &schemas {
        if let Err(e) = teardown_test_schema_internal(&database_url, schema_name).await {
            warn!("Failed to drop orphaned test schema {schema_name}: {e}");
        } else {
            debug!("Dropped orphaned test schema: {schema_name}");
            dropped_count += 1;
        }
    }

    info!("Cleaned up {dropped_count} orphaned test schemas");
    Ok(dropped_count)
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
/// The returned `TestDatabase` wrapper automatically drops the schema when it
/// goes out of scope, preventing accumulation of schemas that consume shared memory.
///
/// # Panics
/// Panics if `DATABASE_URL` is not set in `.env.test` or if database connection fails
#[must_use]
pub async fn setup_test_db() -> TestDatabase {
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

    // Return the TestDatabase wrapper which will automatically clean up the schema
    TestDatabase {
        pool,
        schema_name,
        database_url,
    }
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
