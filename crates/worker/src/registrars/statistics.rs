#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;
use std::sync::Arc;

use crate::context::TaskContext;
use crate::tasks::statistics_collection::StatisticsCollectionTask;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::MaintenanceConfig;
use r_data_core_core::maintenance::MaintenanceTask;
use tokio_cron_scheduler::{Job, JobScheduler};

use super::trait_::TaskRegistrar;

/// Registrar for statistics collection task
pub struct StatisticsCollectionRegistrar;

impl TaskRegistrar for StatisticsCollectionRegistrar {
    async fn register(
        &self,
        scheduler: &JobScheduler,
        pool: PgPool,
        cache_manager: Arc<CacheManager>,
        config: &MaintenanceConfig,
    ) -> r_data_core_core::error::Result<()> {
        let cron = "0 * * * * *".to_string(); // Every minute at second 0
        let pool_clone = pool.clone();
        let cache_manager_clone = cache_manager.clone();
        let license_config = config.license.clone();
        let database_url = config.database.connection_string.clone();

        // Build admin URI from config
        let protocol = if config.api.use_tls { "https" } else { "http" };
        let admin_uri = format!("{protocol}://{}:{}", config.api.host, config.api.port);
        let cors_origins = config.api.cors_origins.clone();

        let admin_uri_clone = admin_uri.clone();
        let cors_origins_clone = cors_origins.clone();
        let cron_clone = cron.clone();
        let database_url_clone = database_url.clone();

        let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
            let pool = pool_clone.clone();
            let cache_manager = cache_manager_clone.clone();
            let license_config = license_config.clone();
            let admin_uri = admin_uri_clone.clone();
            let cors_origins = cors_origins_clone.clone();
            let cron = cron_clone.clone();
            let database_url = database_url_clone.clone();
            Box::pin(async move {
                let task = StatisticsCollectionTask::new(
                    cron,
                    license_config,
                    admin_uri,
                    cors_origins,
                    database_url,
                );
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = task.execute(&context).await {
                    // Silent failure - only print to stdout
                    println!("Statistics collection task failed: {e}");
                }
            })
        })
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to create job: {e}"))
        })?;

        scheduler.add(job).await.map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to add job to scheduler: {e}"))
        })?;

        Ok(())
    }
}
