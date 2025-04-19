use dotenv::dotenv;
use log::debug;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Set up a test database connection
pub async fn setup_test_db() -> PgPool {
    dotenv::from_filename(".env.test").ok();

    // Use the test database URL from the environment or fallback to a default
    let database_url = std::env::var("DATABASE_URL").expect("Failed to get test database URL");

    debug!("Connecting to test database: {}", database_url);

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Drop and recreate the public schema safely
    sqlx::query("DROP SCHEMA IF EXISTS public CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to drop schema");
    
    sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
        .execute(&pool)
        .await
        .expect("Failed to create schema");

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
        "INSERT INTO admin_users (uuid, username, email, password_hash, first_name, last_name, is_active, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)",
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
