#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Manual statistics submission utility for `r_data_core`.
//!
//! This binary allows you to manually trigger statistics submission
//! without waiting for the scheduled hour based on your license key ID.
//!
//! # Usage
//!
//! ```bash
//! # Send statistics (uses environment variables)
//! send_statistics
//!
//! # Show what would be sent (dry-run)
//! send_statistics --dry-run
//! ```
//!
//! # Environment Variables
//!
//! - `DATABASE_URL` - `PostgreSQL` connection string (required)
//! - `LICENSE_KEY` - License key JWT (required)
//! - `STATISTICS_URL` - Statistics API endpoint (required, from license config)
//! - `ADMIN_URI` - Admin URI to report (optional, defaults to empty)
//! - `CORS_ORIGINS` - Comma-separated CORS origins (optional)
//!
//! # Exit Codes
//!
//! - 0: Success
//! - 1: Error (connection failed, invalid configuration, etc.)

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use dotenvy::dotenv;
use r_data_core_core::config::LicenseConfig;
use r_data_core_persistence::StatisticsRepository;
use r_data_core_services::StatisticsService;
use sqlx::postgres::PgPoolOptions;

/// Main entry point for the statistics submission utility
#[tokio::main]
async fn main() -> ExitCode {
    // Load .env file if present
    dotenv().ok();

    // Parse arguments
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    let dry_run = args.iter().any(|a| a == "--dry-run" || a == "-n");

    // Get required environment variables
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("Error: DATABASE_URL environment variable is not set");
        return ExitCode::FAILURE;
    };

    let Ok(license_key) = env::var("LICENSE_KEY") else {
        eprintln!("Error: LICENSE_KEY environment variable is not set");
        return ExitCode::FAILURE;
    };

    let Ok(statistics_url) = env::var("STATISTICS_URL") else {
        eprintln!("Error: STATISTICS_URL environment variable is not set");
        eprintln!("This should be the URL of your statistics API endpoint");
        return ExitCode::FAILURE;
    };

    let admin_uri = env::var("ADMIN_URI").unwrap_or_default();
    let cors_origins: Vec<String> = env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

    println!("Connecting to database...");

    // Create database pool
    let pool = match PgPoolOptions::new()
        .max_connections(2)
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

    // Create license config
    let license_config = LicenseConfig {
        license_key: Some(license_key),
        private_key: None,
        public_key: None,
        verification_url: String::new(), // Not needed for statistics
        statistics_url,
    };

    // Create statistics service
    let repository = Arc::new(StatisticsRepository::new(pool));
    let service = StatisticsService::new(license_config, repository);

    if dry_run {
        println!("[DRY-RUN] Would send statistics to the configured endpoint");
        println!("[DRY-RUN] Admin URI: {admin_uri}");
        println!("[DRY-RUN] CORS Origins: {cors_origins:?}");
        println!("[DRY-RUN] Run without --dry-run to actually send statistics");
        return ExitCode::SUCCESS;
    }

    println!("Collecting and sending statistics...");
    service.collect_and_send(&admin_uri, &cors_origins).await;
    println!("Statistics submission completed.");

    ExitCode::SUCCESS
}

/// Print help information
fn print_help() {
    println!("send_statistics - Manual statistics submission utility for r_data_core");
    println!();
    println!("USAGE:");
    println!("    send_statistics [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Print this help message");
    println!("    -n, --dry-run   Show what would be sent without actually sending");
    println!();
    println!("ENVIRONMENT:");
    println!("    DATABASE_URL     PostgreSQL connection string (required)");
    println!("    LICENSE_KEY      License key JWT (required)");
    println!("    STATISTICS_URL   Statistics API endpoint URL (required)");
    println!("    ADMIN_URI        Admin URI to include in report (optional)");
    println!("    CORS_ORIGINS     Comma-separated CORS origins (optional)");
    println!();
    println!("EXAMPLES:");
    println!("    # Send statistics using environment variables");
    println!("    send_statistics");
    println!();
    println!("    # Preview what would be sent");
    println!("    send_statistics --dry-run");
    println!();
    println!("    # In Docker Compose environment");
    println!("    docker compose exec core /usr/local/bin/send_statistics");
    println!();
    println!("NOTE:");
    println!("    This bypasses the scheduled hour check that normally runs");
    println!("    based on your license key ID. Use this for testing or");
    println!("    when you need to send statistics immediately.");
}
