use std::sync::Arc;

use env_logger;
use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler};

use r_data_core::cache::CacheManager;
use r_data_core::config::MaintenanceConfig;
use r_data_core::entity::version_repository::VersionRepository;
use r_data_core::services::bootstrap::{
    init_cache_manager, init_logger_with_default, init_pg_pool,
};
use r_data_core::services::settings_service::SettingsService;
use r_data_core::system_settings::entity_versioning::EntityVersioningSettings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init logger
    init_logger_with_default("info");

    info!("Starting maintenance worker");

    let cfg = MaintenanceConfig::from_env().expect("Failed to load MaintenanceConfig");

    let pool = init_pg_pool(
        &cfg.database.connection_string,
        cfg.database.max_connections,
    )
    .await?;

    // Cache manager (optionally Redis via config)
    let cache_mgr = init_cache_manager(cfg.cache.clone(), Some(&cfg.redis_url)).await;

    let cron = cfg.cron.clone();

    let scheduler = JobScheduler::new().await?;
    let pool_clone = pool.clone();
    let cache_clone = cache_mgr.clone();

    let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
        let pool = pool_clone.clone();
        let cache = cache_clone.clone();
        Box::pin(async move {
            if let Err(e) = run_prune(pool, cache).await {
                error!("Maintenance prune job failed: {}", e);
            }
        })
    })?;
    scheduler.add(job).await?;

    scheduler.start().await?;
    info!("Maintenance scheduler started with cron '{}'", cron);

    futures::future::pending::<()>().await;
    Ok(())
}

async fn run_prune(
    pool: sqlx::Pool<sqlx::Postgres>,
    cache: Arc<CacheManager>,
) -> anyhow::Result<()> {
    let settings_service = SettingsService::new(pool.clone(), cache);
    let EntityVersioningSettings {
        enabled,
        max_versions,
        max_age_days,
    } = settings_service
        .get_entity_versioning_settings()
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if !enabled {
        info!("Entity versioning disabled; prune skipped");
        return Ok(());
    }

    let version_repo = VersionRepository::new(pool.clone());
    // Prune by age first if configured
    if let Some(days) = max_age_days {
        let _ = version_repo.prune_older_than_days(days).await?;
    }

    // Prune by count (preserve latest N per entity)
    if let Some(keep) = max_versions {
        let _ = version_repo.prune_keep_latest_per_entity(keep).await?;
    }

    info!("Maintenance prune completed");
    Ok(())
}
