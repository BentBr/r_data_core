#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{error, info};
use r_data_core_core::config::load_maintenance_config;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::refresh_token_cleanup::RefreshTokenCleanupTask;
use r_data_core_worker::tasks::version_purger::VersionPurgerTask;
use tokio_cron_scheduler::JobScheduler;

#[tokio::main]
async fn main() -> r_data_core_core::error::Result<()> {
    // Basic logger init
    init_logger_with_default("info");

    info!("Starting maintenance worker");

    let config = match load_maintenance_config() {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {e}");
            return Err(r_data_core_core::error::Error::Config(format!(
                "Failed to load configuration: {e}"
            )));
        }
    };

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to initialize database pool: {e}"))
    })?;

    // Initialize cache manager
    let cache_manager = init_cache_manager(config.cache.clone(), Some(&config.redis_url)).await;

    // Create scheduler
    let scheduler = JobScheduler::new().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to create job scheduler: {e}"))
    })?;

    // Register version purger task
    let version_purger_cron = config.version_purger_cron.clone();
    let pool_clone = pool.clone();
    let cache_manager_clone = cache_manager.clone();
    let job = tokio_cron_scheduler::Job::new_async(
        version_purger_cron.clone().as_str(),
        move |_uuid, _l| {
            let pool = pool_clone.clone();
            let cache_manager = cache_manager_clone.clone();
            let cron = version_purger_cron.clone();
            Box::pin(async move {
                let version_purger = VersionPurgerTask::new(cron);
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = version_purger.execute(&context).await {
                    error!("Version purger task failed: {e}");
                }
            })
        },
    )
    .map_err(|e| r_data_core_core::error::Error::Config(format!("Failed to create job: {e}")))?;
    scheduler.add(job).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to add job to scheduler: {e}"))
    })?;

    // Register refresh token cleanup task
    let refresh_token_cleanup_cron = config.refresh_token_cleanup_cron.clone();
    let pool_clone2 = pool.clone();
    let cache_manager_clone2 = cache_manager.clone();
    let job2 = tokio_cron_scheduler::Job::new_async(
        refresh_token_cleanup_cron.clone().as_str(),
        move |_uuid, _l| {
            let pool = pool_clone2.clone();
            let cache_manager = cache_manager_clone2.clone();
            let cron = refresh_token_cleanup_cron.clone();
            Box::pin(async move {
                let refresh_token_cleanup = RefreshTokenCleanupTask::new(cron);
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = refresh_token_cleanup.execute(&context).await {
                    error!("Refresh token cleanup task failed: {e}");
                }
            })
        },
    )
    .map_err(|e| r_data_core_core::error::Error::Config(format!("Failed to create job: {e}")))?;
    scheduler.add(job2).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to add job to scheduler: {e}"))
    })?;

    info!("Maintenance scheduler started");
    scheduler.start().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to start scheduler: {e}"))
    })?;

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
