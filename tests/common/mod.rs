use log::debug;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Set up a test database connection
pub async fn setup_test_db() -> PgPool {
    dotenv::from_filename(".env.test").ok();

    // Use the test database URL from the environment or fallback to a default
    let database_url = std::env::var("DATABASE_URL").expect("Failed to get test database URL");

    debug!("Connecting to test database: {}", database_url);

    let pool = PgPoolOptions::new()
        // Making sure to only work in one consecutive transaction at any time
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let tables = sqlx::query_scalar!(
        "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname = 'public'"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch table names");

    for table in tables {
        if let Some(table_name) = table {
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS (
                SELECT FROM pg_tables 
                WHERE schemaname = 'public' 
                AND tablename = $1
            )",
                table_name
            )
            .fetch_one(&pool)
            .await
            .expect(&format!("Failed to check if table {} exists", table_name));

            if exists.unwrap_or(false) {
                // Actually truncate the table if it exists
                sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table_name))
                    .execute(&pool)
                    .await
                    .expect(&format!("Failed to truncate table {}", table_name));
            }
        }
    }

    // Optionally, reset sequences
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
    .execute(&pool)
    .await
    .expect("Failed to reset sequences");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    debug!("Test database connection established and migrations applied");

    pool
}

/// Generate a random string for testing
pub fn random_string(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::now_v7())
}

// In tests/common/mod.rs
pub async fn create_test_admin_user(pool: &PgPool) -> Result<Uuid, sqlx::Error> {
    // Check if we already have an admin user
    let existing = sqlx::query!("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_optional(pool)
        .await?;

    if let Some(user) = existing {
        return Ok(user.uuid);
    }

    // Create a new admin user if none exists
    let uuid = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO admin_users (uuid, username, email, password_hash, first_name, last_name, is_active, created_at, updated_at, created_by) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $1)",
        uuid,
        "test_admin",
        "test@example.com",
        "dummy_hash",  // Not secure, but fine for tests
        "Test",
        "Admin",
        true,
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await?;

    Ok(uuid)
}
