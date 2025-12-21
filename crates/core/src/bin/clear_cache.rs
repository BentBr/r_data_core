#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Redis cache clearing utility for `r_data_core`.
//!
//! This binary clears the Redis cache, either entirely or by key prefix.
//! It reads the `REDIS_URL` from environment variables (or .env file).
//!
//! # Usage
//!
//! ```bash
//! # Clear all cache (FLUSHDB)
//! clear_cache --all
//!
//! # Clear cache keys matching a prefix
//! clear_cache --prefix "entity_definitions:"
//!
//! # List keys matching a pattern (dry-run)
//! clear_cache --prefix "entity:" --dry-run
//! ```
//!
//! # Environment Variables
//!
//! - `REDIS_URL` - Redis connection string (required)
//!
//! # Exit Codes
//!
//! - 0: Success
//! - 1: Error (connection failed, invalid arguments, etc.)

use std::env;
use std::process::ExitCode;

use dotenvy::dotenv;
use redis::Client;

/// Main entry point for the cache clearing utility
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

    let clear_all = args.iter().any(|a| a == "--all" || a == "-a");
    let dry_run = args.iter().any(|a| a == "--dry-run" || a == "-n");
    let prefix = get_arg_value(&args, "--prefix").or_else(|| get_arg_value(&args, "-p"));

    // Validate arguments
    if !clear_all && prefix.is_none() {
        eprintln!("Error: Must specify either --all or --prefix <PREFIX>");
        eprintln!();
        eprintln!("Run with --help for usage information.");
        return ExitCode::FAILURE;
    }

    if clear_all && prefix.is_some() {
        eprintln!("Error: Cannot specify both --all and --prefix");
        return ExitCode::FAILURE;
    }

    // Get Redis URL
    let Ok(redis_url) = env::var("REDIS_URL") else {
        eprintln!("Error: REDIS_URL environment variable is not set");
        eprintln!();
        eprintln!("Set it in your environment or in a .env file:");
        eprintln!("  REDIS_URL=redis://localhost:6379");
        return ExitCode::FAILURE;
    };

    println!("Connecting to Redis: {redis_url}");

    // Connect to Redis
    let client = match Client::open(redis_url.as_str()) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error: Failed to create Redis client: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(conn) => {
            println!("Connected successfully.");
            conn
        }
        Err(e) => {
            eprintln!("Error: Failed to connect to Redis: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Execute the requested operation
    if clear_all {
        if dry_run {
            println!("[DRY-RUN] Would clear entire cache (FLUSHDB)");
            return ExitCode::SUCCESS;
        }

        println!("Clearing entire cache...");
        match redis::cmd("FLUSHDB").query_async::<()>(&mut conn).await {
            Ok(()) => {
                println!("Cache cleared successfully.");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error: Failed to clear cache: {e}");
                ExitCode::FAILURE
            }
        }
    } else if let Some(prefix) = prefix {
        match clear_by_prefix(&mut conn, &prefix, dry_run).await {
            Ok(count) => {
                if dry_run {
                    println!("[DRY-RUN] Would delete {count} keys matching prefix '{prefix}'");
                } else {
                    println!("Deleted {count} keys matching prefix '{prefix}'");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        // Should not reach here due to earlier validation
        ExitCode::FAILURE
    }
}

/// Clear cache keys matching a prefix
async fn clear_by_prefix(
    conn: &mut redis::aio::MultiplexedConnection,
    prefix: &str,
    dry_run: bool,
) -> Result<usize, String> {
    let mut deleted = 0;
    let mut cursor = 0u64;
    let pattern = format!("{prefix}*");

    println!("Scanning for keys matching '{pattern}'...");

    loop {
        // Use SCAN to find keys matching the pattern
        let result: (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(conn)
            .await
            .map_err(|e| format!("Failed to scan Redis keys: {e}"))?;

        cursor = result.0;
        let keys = result.1;

        if !keys.is_empty() {
            if dry_run {
                // In dry-run mode, just print the keys
                for key in &keys {
                    println!("  [DRY-RUN] Would delete: {key}");
                }
                deleted += keys.len();
            } else {
                // Delete the keys
                let count: u64 = redis::cmd("DEL")
                    .arg(&keys)
                    .query_async(conn)
                    .await
                    .map_err(|e| format!("Failed to delete Redis keys: {e}"))?;

                for key in &keys {
                    println!("  Deleted: {key}");
                }
                deleted += usize::try_from(count).unwrap_or(0);
            }
        }

        // If cursor is 0, we've scanned all keys
        if cursor == 0 {
            break;
        }
    }

    Ok(deleted)
}

/// Get the value of a command-line argument
fn get_arg_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
        // Also support --prefix=value format
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_string());
        }
    }
    None
}

/// Print help information
fn print_help() {
    println!("clear_cache - Redis cache clearing utility for r_data_core");
    println!();
    println!("USAGE:");
    println!("    clear_cache [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help              Print this help message");
    println!("    -a, --all               Clear entire cache (FLUSHDB)");
    println!("    -p, --prefix <PREFIX>   Clear keys matching prefix");
    println!("    -n, --dry-run           Show what would be deleted without actually deleting");
    println!();
    println!("ENVIRONMENT:");
    println!("    REDIS_URL    Redis connection string (required)");
    println!("                 Example: redis://localhost:6379");
    println!();
    println!("EXAMPLES:");
    println!("    # Clear all cache");
    println!("    REDIS_URL=redis://localhost:6379 clear_cache --all");
    println!();
    println!("    # Clear entity definition cache");
    println!("    clear_cache --prefix \"entity_definitions:\"");
    println!();
    println!("    # Preview what would be deleted");
    println!("    clear_cache --prefix \"api_keys:\" --dry-run");
    println!();
    println!("    # In Docker Compose environment");
    println!("    docker compose exec core /usr/local/bin/clear_cache --all");
    println!();
    println!("COMMON CACHE PREFIXES:");
    println!("    entity_definitions:    Entity definition cache");
    println!("    api_keys:              API key cache");
    println!("    entities:              Entity data cache");
}
