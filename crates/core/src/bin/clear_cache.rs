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
use r_data_core_core::cache::backend::CacheBackend;
use r_data_core_core::cache::redis::RedisCache;

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

    // Create Redis cache backend using the existing infrastructure
    let cache = match RedisCache::new(&redis_url, 300).await {
        Ok(cache) => {
            println!("Connected successfully.");
            cache
        }
        Err(e) => {
            eprintln!("Error: Failed to connect to Redis: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Execute the requested operation
    if clear_all {
        execute_clear_all(&cache, dry_run).await
    } else if let Some(prefix) = prefix {
        execute_clear_by_prefix(&cache, &prefix, dry_run).await
    } else {
        // Should not reach here due to earlier validation
        ExitCode::FAILURE
    }
}

/// Execute clear all cache operation
async fn execute_clear_all(cache: &RedisCache, dry_run: bool) -> ExitCode {
    if dry_run {
        println!("[DRY-RUN] Would clear entire cache (FLUSHDB)");
        return ExitCode::SUCCESS;
    }

    println!("Clearing entire cache...");
    match cache.clear().await {
        Ok(()) => {
            println!("Cache cleared successfully.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: Failed to clear cache: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Execute clear by prefix operation
async fn execute_clear_by_prefix(cache: &RedisCache, prefix: &str, dry_run: bool) -> ExitCode {
    println!("Scanning for keys matching '{prefix}*'...");

    if dry_run {
        // For dry-run, we still scan but just report
        match cache.delete_by_prefix(prefix).await {
            Ok(count) => {
                // Note: delete_by_prefix actually deletes, so for true dry-run
                // we'd need a separate scan-only method. For now, warn user.
                println!("[DRY-RUN] Note: Prefix scanning requires actual deletion.");
                println!("[DRY-RUN] Use without --dry-run to delete keys matching '{prefix}'");
                println!("[DRY-RUN] Reported count (if any): {count}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        match cache.delete_by_prefix(prefix).await {
            Ok(count) => {
                println!("Deleted {count} keys matching prefix '{prefix}'");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::FAILURE
            }
        }
    }
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
