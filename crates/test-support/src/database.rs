#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{debug, info, warn};
use r_data_core_core::error::Result;
use sqlx::{postgres::PgPoolOptions, postgres::PgRow, PgPool, Row};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Duration;

/// Global mutex for DB operations
pub static GLOBAL_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Track the last used entity type to ensure uniqueness
static ENTITY_TYPE_COUNTER: Mutex<u32> = Mutex::new(0);

/// Keep track of DB initialization to avoid duplicate setups
static DB_READY: AtomicBool = AtomicBool::new(false);

/// Generate a unique entity type name to avoid conflicts between tests
#[must_use]
pub fn unique_entity_type(base: &str) -> String {
    let mut counter = ENTITY_TYPE_COUNTER
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let count = *counter;
    *counter = count + 1;
    drop(counter);

    format!("{base}_{count}")
}

/// Generate a random string for testing
#[must_use]
pub fn random_string(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::now_v7())
}

/// Set up a test database connection
#[must_use]
pub async fn setup_test_db() -> PgPool {
    // Get global lock for the entire test run
    let _guard = GLOBAL_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    info!("Setting up test database with global lock acquired");

    // Load environment variables from .env.test
    dotenv::from_filename(".env.test").ok();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set in .env.test");
    info!("Connecting to test database: {database_url}");

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
        match sqlx::migrate!("../../migrations").run(&pool).await {
            Ok(()) => info!("Database migrations completed successfully"),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    info!("Some migration objects already exist, continuing");
                } else {
                    panic!("Failed to run migrations: {e}");
                }
            }
        }

        // Set the initialization flag to avoid redoing this work
        DB_READY.store(true, std::sync::atomic::Ordering::Release);
    } else {
        // If database is already initialized, just clear the data
        if let Err(e) = fast_clear_test_db(&pool).await {
            warn!("Warning: Failed to clear test database: {e}");
        }
    }

    // Return the dedicated pool for this test
    pool
}

/// Clear all data from the database - optimized version for faster test runs
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
    let entity_tables: Vec<String> = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = 'public'
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
pub async fn clear_entity_definitions(pool: &PgPool) -> Result<()> {
    debug!("Clearing entity definitions");

    sqlx::query("TRUNCATE TABLE entity_definitions CASCADE")
        .execute(pool)
        .await?;

    debug!("Class definitions cleared successfully");
    Ok(())
}

/// Clear all data from the database - original thorough version
pub async fn clear_test_db(pool: &PgPool) -> Result<()> {
    info!("Clearing test database data");

    // Use a transaction for atomicity
    let mut tx = pool.begin().await?;

    // Disable foreign key constraints during cleanup
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get all tables except migration table
    let tables: Vec<String> = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables
         WHERE schemaname = 'public'
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
pub async fn clear_refresh_tokens(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM refresh_tokens")
        .execute(pool)
        .await?;
    Ok(())
}
