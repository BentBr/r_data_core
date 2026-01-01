#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::too_many_lines)] // Maintenance binary needs to register multiple tasks

use log::{error, info};
use r_data_core_core::config::load_maintenance_config;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_services::LicenseService;
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::license_verification::LicenseVerificationTask;
use r_data_core_worker::tasks::refresh_token_cleanup::RefreshTokenCleanupTask;
use r_data_core_worker::tasks::statistics_collection::StatisticsCollectionTask;
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

    // Verify license on startup
    let license_service = LicenseService::new(config.license.clone(), cache_manager.clone());
    license_service
        .verify_license_on_startup("maintenance")
        .await;

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

    // Register license verification task (runs every hour, checks conditions internally)
    let license_verification_cron = "0 * * * * *".to_string(); // Every hour at minute 0
    let license_verification_cron_clone = license_verification_cron.clone();
    let pool_clone3 = pool.clone();
    let cache_manager_clone3 = cache_manager.clone();
    let license_config_clone = config.license.clone();
    let job3 = tokio_cron_scheduler::Job::new_async(
        license_verification_cron.as_str(),
        move |_uuid, _l| {
            let pool = pool_clone3.clone();
            let cache_manager = cache_manager_clone3.clone();
            let license_config = license_config_clone.clone();
            let cron = license_verification_cron_clone.clone();
            Box::pin(async move {
                let license_verification = LicenseVerificationTask::new(cron, license_config);
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = license_verification.execute(&context).await {
                    error!("License verification task failed: {e}");
                }
            })
        },
    )
    .map_err(|e| r_data_core_core::error::Error::Config(format!("Failed to create job: {e}")))?;
    scheduler.add(job3).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to add job to scheduler: {e}"))
    })?;

    // Register statistics collection task (runs every hour, checks conditions internally)
    let statistics_cron = "0 * * * * *".to_string(); // Every hour at minute 0
    let pool_clone4 = pool.clone();
    let cache_manager_clone4 = cache_manager.clone();
    let license_config_clone2 = config.license.clone();

    // Build admin URI from config
    let protocol = if config.api.use_tls { "https" } else { "http" };
    let admin_uri = format!("{protocol}://{}:{}", config.api.host, config.api.port);
    let cors_origins = config.api.cors_origins.clone();

    let admin_uri_clone = admin_uri.clone();
    let cors_origins_clone = cors_origins.clone();
    let job4 =
        tokio_cron_scheduler::Job::new_async(statistics_cron.clone().as_str(), move |_uuid, _l| {
            let pool = pool_clone4.clone();
            let cache_manager = cache_manager_clone4.clone();
            let license_config = license_config_clone2.clone();
            let admin_uri = admin_uri_clone.clone();
            let cors_origins = cors_origins_clone.clone();
            let cron = statistics_cron.clone();
            Box::pin(async move {
                let statistics =
                    StatisticsCollectionTask::new(cron, license_config, admin_uri, cors_origins);
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = statistics.execute(&context).await {
                    // Silent failure - only print to stdout
                    println!("Statistics collection task failed: {e}");
                }
            })
        })
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to create job: {e}"))
        })?;
    scheduler.add(job4).await.map_err(|e| {
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
