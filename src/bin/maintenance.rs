use std::sync::Arc;
use std::time::Instant;

use log::{error, info};
use tokio_cron_scheduler::{Job, JobScheduler};

use r_data_core::config::MaintenanceConfig;
use r_data_core::maintenance::tasks::VersionPurgerTask;
use r_data_core::maintenance::{MaintenanceTask, TaskContext};
use r_data_core::services::bootstrap::{
    init_cache_manager, init_logger_with_default, init_pg_pool,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init logger
    init_logger_with_default("info");

    info!("========================================");
    info!("Starting maintenance worker");
    info!("========================================");

    let cfg = MaintenanceConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load MaintenanceConfig: {}", e))?;

    info!("Maintenance worker configuration loaded:");
    info!(
        "  - Database: {} (max_connections: {})",
        cfg.database
            .connection_string
            .split('@')
            .last()
            .unwrap_or("***"),
        cfg.database.max_connections
    );
    info!(
        "  - Cache: enabled={}, ttl={}s",
        cfg.cache.enabled, cfg.cache.ttl
    );
    info!("  - Default cron: {}", cfg.cron);

    let pool = init_pg_pool(
        &cfg.database.connection_string,
        cfg.database.max_connections,
    )
    .await?;

    info!("Database connection pool initialized");

    // Cache manager (optionally Redis via config)
    let cache_mgr = init_cache_manager(cfg.cache.clone(), Some(&cfg.redis_url)).await;
    info!("Cache manager initialized");

    // Create task context
    let task_context = Arc::new(TaskContext {
        pool: pool.clone(),
        cache: cache_mgr.clone(),
    });

    // Initialize scheduler
    let scheduler = JobScheduler::new().await?;
    info!("Job scheduler initialized");

    // Register all maintenance tasks
    info!("Discovering and registering maintenance tasks...");

    // Register version purger task
    {
        let task = VersionPurgerTask::new(cfg.version_purger_cron.clone());
        let task_name = task.name();
        let cron_expr = task.cron();

        info!("Registering task '{}' with cron '{}'", task_name, cron_expr);

        // Validate cron expression
        r_data_core::utils::cron::validate_cron(cron_expr).map_err(|e| {
            anyhow::anyhow!(
                "Invalid cron expression '{}' for task '{}': {}",
                cron_expr,
                task_name,
                e
            )
        })?;

        let context = task_context.clone();
        let task_name_log = task_name.to_string();
        let cron_expr_str = cron_expr.to_string();
        let version_purger_cron = cfg.version_purger_cron.clone();

        // Create job for this task
        let job = Job::new_async(&cron_expr_str, move |_uuid, _l| {
            let context = context.clone();
            let task_name = task_name_log.clone();
            let version_purger_cron = version_purger_cron.clone();
            Box::pin(async move {
                let start_time = Instant::now();
                info!("[{}] Task execution started", task_name);

                // Create a new task instance for this execution
                let task = VersionPurgerTask::new(version_purger_cron);
                match task.execute(&context).await {
                    Ok(()) => {
                        let duration = start_time.elapsed();
                        info!(
                            "[{}] Task execution completed successfully in {:?}",
                            task_name, duration
                        );
                    }
                    Err(e) => {
                        let duration = start_time.elapsed();
                        error!(
                            "[{}] Task execution failed after {:?}: {}",
                            task_name, duration, e
                        );
                    }
                }
            })
        })?;

        let job_id = scheduler.add(job).await?;
        info!(
            "Task '{}' registered successfully with job ID: {}",
            task_name, job_id
        );
    }

    // Add more task registrations here as they are implemented
    // Example for future tasks:
    // {
    //     let task = AnotherTask::new();
    //     let task_name = task.name();
    //     let cron_expr = task.cron();
    //     // ... similar registration code ...
    // }

    // Start the scheduler
    scheduler.start().await?;
    info!("========================================");
    info!("Maintenance scheduler started");
    info!("All tasks are now scheduled and running");
    info!("========================================");

    // Keep the process running
    futures::future::pending::<()>().await;
    Ok(())
}
