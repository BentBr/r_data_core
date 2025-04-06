use dotenv::dotenv;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::env;

use r_data_core::db::run_migrations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Read database URL from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!("Connecting to database...");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    info!("Connected to database successfully");
    info!("Running migrations...");

    // Run migrations
    match run_migrations(&pool).await {
        Ok(_) => {
            info!("Migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to run migrations: {}", e);
            Err(Box::new(e))
        }
    }
}
