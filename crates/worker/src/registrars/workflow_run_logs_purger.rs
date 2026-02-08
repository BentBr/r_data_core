#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::error;
use sqlx::PgPool;
use std::sync::Arc;

use crate::context::TaskContext;
use crate::tasks::workflow_run_logs_purger::WorkflowRunLogsPurgerTask;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::MaintenanceConfig;
use r_data_core_core::maintenance::MaintenanceTask;
use tokio_cron_scheduler::{Job, JobScheduler};

use super::trait_::TaskRegistrar;

/// Registrar for workflow run logs purger task
pub struct WorkflowRunLogsPurgerRegistrar;

impl TaskRegistrar for WorkflowRunLogsPurgerRegistrar {
    async fn register(
        &self,
        scheduler: &JobScheduler,
        pool: PgPool,
        cache_manager: Arc<CacheManager>,
        config: &MaintenanceConfig,
    ) -> r_data_core_core::error::Result<()> {
        let cron = config.workflow_run_logs_purger_cron.clone();
        let pool_clone = pool.clone();
        let cache_manager_clone = cache_manager.clone();
        let cron_clone = cron.clone();

        let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
            let pool = pool_clone.clone();
            let cache_manager = cache_manager_clone.clone();
            let cron = cron_clone.clone();
            Box::pin(async move {
                let task = WorkflowRunLogsPurgerTask::new(cron);
                let context = TaskContext::with_cache(pool, cache_manager);
                if let Err(e) = task.execute(&context).await {
                    error!("Workflow run logs purger task failed: {e}");
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
