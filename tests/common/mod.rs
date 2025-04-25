use lazy_static::lazy_static;
use log::debug;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Mutex;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::OnceCell;
use uuid::Uuid;

// Global pool to ensure only one connection is used for all tests
static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

// Use a Mutex to guard DB initialization instead of Once
lazy_static! {
    static ref DB_INIT: Mutex<bool> = Mutex::new(false);
}

/// Set up a test database connection - returns a shared global pool
pub async fn setup_test_db() -> PgPool {
    // Get or initialize the global pool
    DB_POOL
        .get_or_init(|| async {
            dotenv::from_filename(".env.test").ok();

            // Use the test database URL from the environment
            let database_url =
                std::env::var("DATABASE_URL").expect("Failed to get test database URL");
            debug!("Connecting to test database: {}", database_url);

            // Create a connection pool with EXACTLY ONE connection
            // Set idle_timeout to None to keep the connection alive
            // Set acquire_timeout to a reasonable value to fail fast if can't acquire
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .min_connections(1)
                .idle_timeout(None)
                .acquire_timeout(Duration::from_secs(5))
                .connect(&database_url)
                .await
                .expect("Failed to connect to test database");

            // Run migrations only once for all tests using a mutex guard
            let mut initialized = DB_INIT.lock().unwrap();
            if !*initialized {
                debug!("Running migrations for test database");
                sqlx::migrate!("./migrations")
                    .run(&pool)
                    .await
                    .expect("Failed to run migrations");
                *initialized = true;
                debug!("Migrations complete");
            }

            pool
        })
        .await
        .clone()
}

/// Clear all data from the database
pub async fn clear_test_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Use a transaction to ensure all operations are atomic
    debug!("Starting database cleanup");
    let mut tx = pool.begin().await?;

    // First disable all triggers so we can truncate without constraint errors
    debug!("Disabling triggers for truncation");
    sqlx::query("SET session_replication_role = 'replica'")
        .execute(&mut *tx)
        .await?;

    // Get all tables except migration history
    let tables = sqlx::query_scalar!(
        "SELECT tablename FROM pg_catalog.pg_tables 
         WHERE schemaname = 'public' 
         AND tablename != 'schema_migrations'" // Avoid truncating migration history
    )
    .fetch_all(&mut *tx)
    .await?;

    // Truncate all tables in one statement to avoid foreign key issues
    debug!("Truncating all tables");
    if !tables.is_empty() {
        let table_list = tables
            .iter()
            .filter_map(|t| t.as_deref()) // Use as_deref() instead of as_ref()
            .map(|t| format!("\"{}\"", t))
            .collect::<Vec<_>>()
            .join(", ");

        if !table_list.is_empty() {
            let truncate_sql = format!("TRUNCATE TABLE {} CASCADE", table_list);
            debug!("Executing: {}", truncate_sql);
            sqlx::query(&truncate_sql).execute(&mut *tx).await?;
        }
    }

    // Reset sequences
    debug!("Resetting sequences");
    sqlx::query(
        r#"
        DO $$ DECLARE
            seq RECORD;
        BEGIN
            FOR seq IN
                SELECT c.oid::regclass::text AS seqname
                FROM pg_class c
                JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind = 'S' AND n.nspname = 'public'
            LOOP
                EXECUTE 'ALTER SEQUENCE ' || seq.seqname || ' RESTART WITH 1';
            END LOOP;
        END $$;
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Re-enable triggers
    debug!("Re-enabling triggers");
    sqlx::query("SET session_replication_role = 'origin'")
        .execute(&mut *tx)
        .await?;

    // Commit the transaction
    debug!("Committing transaction");
    tx.commit().await?;

    debug!("Database cleared successfully");
    Ok(())
}

/// Generate a random string for testing
pub fn random_string(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::now_v7())
}

/// Create a test admin user with a guaranteed unique username
pub async fn create_test_admin_user(pool: &PgPool) -> Result<Uuid, sqlx::Error> {
    // Generate truly unique username with UUID
    let uuid = Uuid::now_v7();
    let username = format!("test_admin_{}", uuid.simple());
    let email = format!("test_{}@example.com", uuid.simple());

    // Use a transaction to ensure atomicity
    let mut tx = pool.begin().await?;

    // First check if user already exists (unlikely with UUID but ensures idempotence)
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM admin_users WHERE username = $1 OR email = $2",
        &username,
        &email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0)
        > 0;

    if exists {
        debug!("User already exists, returning UUID: {}", uuid);
        tx.commit().await?;
        return Ok(uuid);
    }

    // Create a new admin user
    debug!("Creating test admin user: {}", username);
    sqlx::query!(
        "INSERT INTO admin_users (uuid, username, email, password_hash, first_name, last_name, is_active, created_at, updated_at, created_by) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $1)",
        uuid,
        username,
        email,
        "dummy_hash",  // Not secure, but fine for tests
        "Test",
        "User",
        true,
        OffsetDateTime::now_utc()
    )
    .execute(&mut *tx)
    .await?;

    // Commit the transaction
    debug!("Committing new admin user transaction");
    tx.commit().await?;

    debug!("Created test admin user with UUID: {}", uuid);
    Ok(uuid)
}
