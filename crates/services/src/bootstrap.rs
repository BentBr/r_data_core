#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use log::info;
use sqlx::postgres::PgPoolOptions;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;

/// Initialize logger with default level
pub fn init_logger_with_default(default_level: &str) {
    let env = env_logger::Env::new().default_filter_or(default_level);
    env_logger::Builder::from_env(env)
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .format_module_path(true)
        .format_target(true)
        .init();
}

/// Initialize `PostgreSQL` connection pool
///
/// # Errors
/// Returns an error if the connection pool cannot be created
pub async fn init_pg_pool(
    connection_string: &str,
    max_connections: u32,
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(connection_string)
        .await?;
    Ok(pool)
}

/// Initialize cache manager with optional Redis support
///
/// # Errors
/// Returns an error if Redis connection fails (falls back to in-memory)
pub async fn init_cache_manager(
    cache_cfg: CacheConfig,
    redis_url: Option<&str>,
) -> Arc<CacheManager> {
    let manager = match redis_url {
        Some(url) if !url.is_empty() => {
            match CacheManager::new(cache_cfg.clone()).with_redis(url).await {
                Ok(m) => {
                    info!("Cache manager initialized with Redis");
                    m
                }
                Err(e) => {
                    info!("Failed to initialize Redis cache ({e}), using in-memory");
                    CacheManager::new(cache_cfg)
                }
            }
        }
        _ => CacheManager::new(cache_cfg),
    };
    Arc::new(manager)
}
