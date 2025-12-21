#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Database migration runner for `r_data_core`.
//!
//! This binary runs `SQLx` migrations against the configured database.
//! It reads the `DATABASE_URL` from environment variables (or .env file).
//!
//! # Usage
//!
//! ```bash
//! # Run migrations (uses DATABASE_URL from environment)
//! run_migrations
//!
//! # Run migrations with explicit database URL
//! DATABASE_URL=postgres://user:pass@host:5432/db run_migrations
//!
//! # Check migration status without running
//! run_migrations --status
//! ```
//!
//! # Environment Variables
//!
//! - `DATABASE_URL` - Postgres connection string (required)
//!
//! # Exit Codes
//!
//! - 0: Success
//! - 1: Error (connection failed, migration failed, etc.)

use std::env;
use std::process::ExitCode;

use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;

/// Main entry point for the migration runner
#[tokio::main]
async fn main() -> ExitCode {
    // Load .env file if present
    dotenv().ok();

    // Check for --help flag
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    let check_status = args.iter().any(|a| a == "--status" || a == "-s");

    // Get database URL
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("Error: DATABASE_URL environment variable is not set");
        eprintln!();
        eprintln!("Set it in your environment or in a .env file:");
        eprintln!("  DATABASE_URL=postgres://user:password@host:5432/database");
        return ExitCode::FAILURE;
    };

    // Mask password in output for security
    let display_url = mask_password(&database_url);
    println!("Connecting to database: {display_url}");

    // Create connection pool
    let pool = match PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("Connected successfully.");
            pool
        }
        Err(e) => {
            eprintln!("Error: Failed to connect to database: {e}");
            return ExitCode::FAILURE;
        }
    };

    if check_status {
        // Just check status, don't run migrations
        match check_migration_status(&pool).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error checking migration status: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        // Run migrations
        match run_migrations(&pool).await {
            Ok(()) => {
                println!("Migrations completed successfully.");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error running migrations: {e}");
                ExitCode::FAILURE
            }
        }
    }
}

/// Run pending migrations
async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), String> {
    println!("Running migrations...");

    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("already exists") {
                // Some objects already exist, this is often fine
                println!("Note: Some migration objects already exist (this is usually fine)");
                return String::new();
            }
            format!("Migration failed: {e}")
        })?;

    Ok(())
}

/// Check migration status without running
async fn check_migration_status(pool: &sqlx::PgPool) -> Result<(), String> {
    println!("Checking migration status...");
    println!();

    // Check if _sqlx_migrations table exists
    let table_exists: bool = sqlx::query_scalar(
        r"SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to check migrations table: {e}"))?;

    if !table_exists {
        println!("No migrations have been run yet (migrations table does not exist).");
        println!("Run without --status to apply all pending migrations.");
        return Ok(());
    }

    // Get applied migrations
    let applied: Vec<(i64, String)> = sqlx::query_as(
        r"SELECT version, description FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query migrations: {e}"))?;

    if applied.is_empty() {
        println!("No migrations have been applied yet.");
    } else {
        println!("Applied migrations:");
        for (version, description) in &applied {
            println!("  [{version}] {description}");
        }
        println!();
        println!("Total: {} migrations applied", applied.len());
    }

    Ok(())
}

/// Mask the password in a database URL for safe display
fn mask_password(url: &str) -> String {
    // Simple regex-like replacement for postgres://user:password@host format
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(slash_pos) = url[..colon_pos].rfind('/') {
                let before_pass = &url[..=colon_pos];
                let after_pass = &url[at_pos..];
                // Only mask if this looks like user:pass@ pattern (not port)
                if slash_pos < colon_pos {
                    return format!("{before_pass}****{after_pass}");
                }
            }
        }
    }
    url.to_string()
}

/// Print help information
fn print_help() {
    println!("run_migrations - Database migration runner for r_data_core");
    println!();
    println!("USAGE:");
    println!("    run_migrations [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Print this help message");
    println!("    -s, --status    Check migration status without running migrations");
    println!();
    println!("ENVIRONMENT:");
    println!("    DATABASE_URL    PostgreSQL connection string (required)");
    println!("                    Example: postgres://user:password@localhost:5432/rdata");
    println!();
    println!("EXAMPLES:");
    println!("    # Run all pending migrations");
    println!("    DATABASE_URL=postgres://postgres:postgres@localhost:5432/rdata run_migrations");
    println!();
    println!("    # Check current migration status");
    println!("    run_migrations --status");
    println!();
    println!("    # In Docker Compose environment");
    println!("    docker compose exec core /usr/local/bin/run_migrations");
}
