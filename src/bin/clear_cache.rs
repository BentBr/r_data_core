use clap::{App, SubCommand};
use dotenv::dotenv;
use log::info;
use r_data_core::cache::CacheManager;
use r_data_core::config::AppConfig;
use std::process;

#[tokio::main]
async fn main() {
    // Load .env file if present
    dotenv().ok();

    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let matches = App::new("clear_cache")
        .about("Clear cache entries for entity definitions and API keys")
        .subcommand(
            SubCommand::with_name("entity-definitions")
                .about("Clear all entity definition cache entries"),
        )
        .subcommand(SubCommand::with_name("api-keys").about("Clear all API key cache entries"))
        .subcommand(
            SubCommand::with_name("all")
                .about("Clear all cache entries (entity definitions and API keys)"),
        )
        .get_matches();

    // Load configuration
    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Initialize cache manager
    let cache_manager = if config.cache.enabled {
        let manager = CacheManager::new(config.cache.clone());

        // Try to connect to Redis if URL is provided
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            if !redis_url.is_empty() {
                match manager.with_redis(&redis_url).await {
                    Ok(m) => {
                        info!("Connected to Redis cache");
                        std::sync::Arc::new(m)
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to connect to Redis: {}. Using in-memory cache only.",
                            e
                        );
                        std::sync::Arc::new(CacheManager::new(config.cache.clone()))
                    }
                }
            } else {
                info!("No Redis URL provided, using in-memory cache only");
                std::sync::Arc::new(manager)
            }
        } else {
            info!("No Redis URL provided, using in-memory cache only");
            std::sync::Arc::new(manager)
        }
    } else {
        eprintln!("Cache is disabled in configuration");
        process::exit(1);
    };

    // Execute the command
    match matches.subcommand() {
        Some(("entity-definitions", _)) => {
            info!("Clearing entity definition cache entries...");
            match cache_manager.delete_by_prefix("entity_def:").await {
                Ok(count) => {
                    info!("Cleared {} entity definition cache entries", count);
                }
                Err(e) => {
                    eprintln!("Error clearing entity definition cache: {}", e);
                    process::exit(1);
                }
            }
        }
        Some(("api-keys", _)) => {
            info!("Clearing API key cache entries...");
            match cache_manager.delete_by_prefix("api_key:").await {
                Ok(count) => {
                    info!("Cleared {} API key cache entries", count);
                }
                Err(e) => {
                    eprintln!("Error clearing API key cache: {}", e);
                    process::exit(1);
                }
            }
        }
        Some(("all", _)) => {
            info!("Clearing all cache entries...");
            let mut total = 0;

            match cache_manager.delete_by_prefix("entity_def:").await {
                Ok(count) => {
                    info!("Cleared {} entity definition cache entries", count);
                    total += count;
                }
                Err(e) => {
                    eprintln!("Error clearing entity definition cache: {}", e);
                }
            }

            match cache_manager.delete_by_prefix("api_key:").await {
                Ok(count) => {
                    info!("Cleared {} API key cache entries", count);
                    total += count;
                }
                Err(e) => {
                    eprintln!("Error clearing API key cache: {}", e);
                }
            }

            info!("Total cache entries cleared: {}", total);
        }
        _ => {
            eprintln!("No command specified. Use --help for usage information.");
            process::exit(1);
        }
    }

    info!("Cache clearing completed successfully");
}
