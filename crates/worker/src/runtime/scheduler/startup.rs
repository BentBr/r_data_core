#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::info;
use tokio_cron_scheduler::JobScheduler;

use crate::runtime::WorkerBootstrap;

use super::jobs::{schedule_workflow_job, ScheduleWorkflowJobConfig};
use super::reconcile::spawn_reconcile_task;

pub async fn start_scheduler(
    bootstrap: &WorkerBootstrap,
) -> r_data_core_core::error::Result<JobScheduler> {
    let scheduler = bootstrap.scheduler.clone();

    {
        let workflows = bootstrap.repo.list_scheduled_consumers().await?;
        for (workflow_id, cron) in workflows {
            let job_cfg = ScheduleWorkflowJobConfig {
                pool: bootstrap.runtime.pool.clone(),
                cache_manager: bootstrap.runtime.cache_manager.clone(),
                outbox_fetch_enabled_default: bootstrap.runtime.outbox_fetch_enabled_default,
                outbox_push_enabled_default: bootstrap.runtime.outbox_push_enabled_default,
                queue: bootstrap.runtime.queue.clone(),
                outbox_repo: bootstrap.runtime.outbox_repo.clone(),
                outbox_retry_policy: bootstrap.runtime.outbox_retry_policy,
            };
            let job_id =
                schedule_workflow_job(scheduler.clone(), workflow_id, cron.clone(), job_cfg)
                    .await?;
            bootstrap
                .scheduled_workflows
                .lock()
                .await
                .insert(workflow_id, (job_id, cron));
        }
    }

    scheduler.start().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to start scheduler: {e}"))
    })?;
    info!("Worker scheduler started");

    spawn_reconcile_task(bootstrap);
    Ok(scheduler)
}
