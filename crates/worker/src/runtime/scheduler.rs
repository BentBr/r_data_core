#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;
use std::sync::Arc;

use log::info;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::workflow::outbox::OutboxRetryPolicy;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};

use crate::runtime::WorkerBootstrap;

async fn schedule_workflow_job(
    scheduler: JobScheduler,
    workflow_id: Uuid,
    cron: String,
    pool: sqlx::PgPool,
    queue: Arc<r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue>,
    outbox_repo: Option<Arc<r_data_core_persistence::OutboxRepository>>,
    outbox_retry_policy: Option<OutboxRetryPolicy>,
) -> r_data_core_core::error::Result<Uuid> {
    let pool_clone = pool.clone();
    let cron_clone = cron.clone();
    let job = Job::new_async(cron_clone.as_str(), move |_uuid, _l| {
        let pool = pool_clone.clone();
        let queue = queue.clone();
        let outbox_repo = outbox_repo.clone();
        Box::pin(async move {
            info!("Schedule: creating run and enqueueing fetch job for workflow {workflow_id}");
            let external_trigger_id = Uuid::now_v7();
            let workflow_service = {
                let base = WorkflowService::new(Arc::new(WorkflowRepositoryAdapter::new(
                    WorkflowRepository::new(pool.clone()),
                )));
                if let Some(outbox_repo) = outbox_repo.clone() {
                    let base = base.with_outbox_repository(outbox_repo);
                    if let Some(policy) = outbox_retry_policy {
                        base.with_outbox_retry_policy(policy)
                    } else {
                        base
                    }
                } else {
                    base
                }
            };
            let _ = workflow_service
                .enqueue_run_for_fetch(workflow_id, queue.as_ref(), Some(external_trigger_id))
                .await;
        })
    })
    .map_err(|e| r_data_core_core::error::Error::Config(format!("Failed to create job: {e}")))?;
    let job_id = scheduler.add(job).await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to add job to scheduler: {e}"))
    })?;
    Ok(job_id)
}

pub(crate) async fn start_scheduler(
    bootstrap: &WorkerBootstrap,
) -> r_data_core_core::error::Result<JobScheduler> {
    let scheduler = bootstrap.scheduler.clone();

    {
        let workflows = bootstrap.repo.list_scheduled_consumers().await?;
        for (workflow_id, cron) in workflows {
            let job_id = schedule_workflow_job(
                scheduler.clone(),
                workflow_id,
                cron.clone(),
                bootstrap.runtime.pool.clone(),
                bootstrap.runtime.queue.clone(),
                bootstrap.runtime.outbox_repo.clone(),
                bootstrap.runtime.outbox_retry_policy,
            )
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

fn spawn_reconcile_task(bootstrap: &WorkerBootstrap) {
    let scheduler_clone = bootstrap.scheduler.clone();
    let repo_clone = WorkflowRepository::new(bootstrap.runtime.pool.clone());
    let pool_clone = bootstrap.runtime.pool.clone();
    let scheduled_map = bootstrap.scheduled_workflows.clone();
    let queue_for_reconcile = bootstrap.runtime.queue.clone();
    let outbox_repo_for_reconcile = bootstrap.runtime.outbox_repo.clone();
    let outbox_retry_policy = bootstrap.runtime.outbox_retry_policy;
    let interval_secs = bootstrap.runtime.job_queue_update_interval_secs;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            if let Ok(db_workflows) = repo_clone.list_scheduled_consumers().await {
                let mut map = scheduled_map.lock().await;
                let current_set: HashMap<Uuid, String> = db_workflows
                    .iter()
                    .map(|(wf_id, cron)| (*wf_id, cron.clone()))
                    .collect();
                let existing_set: HashMap<Uuid, String> = map
                    .iter()
                    .map(|(wf_id, (_job_id, cron))| (*wf_id, cron.clone()))
                    .collect();
                let (wf_to_remove, wf_to_add) =
                    r_data_core_services::compute_reconcile_actions(&existing_set, &current_set);
                for wf_id in wf_to_remove {
                    if let Some((job_id, _)) = map.get(&wf_id) {
                        let _ = scheduler_clone.remove(job_id).await;
                    }
                    map.remove(&wf_id);
                }
                for (wf_id, cron) in wf_to_add {
                    if let Ok(job_id) = schedule_workflow_job(
                        scheduler_clone.clone(),
                        wf_id,
                        cron.clone(),
                        pool_clone.clone(),
                        queue_for_reconcile.clone(),
                        outbox_repo_for_reconcile.clone(),
                        outbox_retry_policy,
                    )
                    .await
                    {
                        map.insert(wf_id, (job_id, cron));
                    }
                }
            }
        }
    });
}
