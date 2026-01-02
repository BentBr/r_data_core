#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{error, info};
use r_data_core_core::config::load_maintenance_config;
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_services::LicenseService;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::MaintenanceConfig;
use r_data_core_worker::registrars::{
    LicenseVerificationRegistrar, RefreshTokenCleanupRegistrar, StatisticsCollectionRegistrar,
    TaskRegistrar, VersionPurgerRegistrar,
};

/// Initialize the maintenance scheduler with all registered tasks
///
/// # Arguments
/// * `config` - Maintenance configuration
/// * `pool` - Database connection pool
/// * `cache_manager` - Cache manager
///
/// # Errors
/// Returns an error if scheduler initialization or task registration fails
async fn init_scheduler(
    config: &MaintenanceConfig,
    pool: PgPool,
    cache_manager: Arc<CacheManager>,
) -> r_data_core_core::error::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to create job scheduler: {e}"))
    })?;

    // Register all tasks using the strategy pattern
    VersionPurgerRegistrar
        .register(&scheduler, pool.clone(), cache_manager.clone(), config)
        .await?;
    RefreshTokenCleanupRegistrar
        .register(&scheduler, pool.clone(), cache_manager.clone(), config)
        .await?;
    LicenseVerificationRegistrar
        .register(&scheduler, pool.clone(), cache_manager.clone(), config)
        .await?;
    StatisticsCollectionRegistrar
        .register(&scheduler, pool.clone(), cache_manager.clone(), config)
        .await?;

    Ok(scheduler)
}

/// Initialize the application (logger, config, database, cache)
///
/// # Errors
/// Returns an error if initialization fails
async fn init_application(
) -> r_data_core_core::error::Result<(MaintenanceConfig, PgPool, Arc<CacheManager>)> {
    init_logger_with_default("info");
    info!("Starting maintenance worker");

    let config = load_maintenance_config().map_err(|e| {
        error!("Failed to load configuration: {e}");
        r_data_core_core::error::Error::Config(format!("Failed to load configuration: {e}"))
    })?;

    info!("Configuration loaded successfully");

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to initialize database pool: {e}"))
    })?;

    let cache_manager = init_cache_manager(config.cache.clone(), Some(&config.redis_url)).await;

    Ok((config, pool, cache_manager))
}

/// Verify license on startup
///
/// # Arguments
/// * `config` - Maintenance configuration
/// * `cache_manager` - Cache manager
async fn verify_license_on_startup(config: &MaintenanceConfig, cache_manager: Arc<CacheManager>) {
    let license_service = LicenseService::new(config.license.clone(), cache_manager);
    license_service
        .verify_license_on_startup("maintenance")
        .await;
}

#[tokio::main]
async fn main() -> r_data_core_core::error::Result<()> {
    let (config, pool, cache_manager) = init_application().await?;

    verify_license_on_startup(&config, cache_manager.clone()).await;

    let scheduler = init_scheduler(&config, pool, cache_manager).await?;

    info!("Maintenance scheduler started");
    scheduler.start().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to start scheduler: {e}"))
    })?;

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
