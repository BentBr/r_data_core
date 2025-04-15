use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Load environment variables
    dotenv().ok();

    // Get database URL from environment variables
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("Migrations completed successfully!");

    Ok(())
}
