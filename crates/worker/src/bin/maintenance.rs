#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{error, info};
use r_data_core_core::config::load_maintenance_config;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::version_purger::VersionPurgerTask;
use tokio_cron_scheduler::JobScheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            return Err(anyhow::anyhow!("Failed to load configuration: {e}"));
        }
    };

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await?;

    // Initialize cache manager
    let cache_manager = init_cache_manager(config.cache.clone(), Some(&config.redis_url)).await;

    // Create scheduler
    let scheduler = JobScheduler::new().await?;

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
    )?;
    scheduler.add(job).await?;

    info!("Maintenance scheduler started");
    scheduler.start().await?;

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
