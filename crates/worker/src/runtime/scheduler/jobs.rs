#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use log::info;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::workflow::outbox::OutboxRetryPolicy;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

pub(super) async fn schedule_workflow_job(
    scheduler: JobScheduler,
    workflow_id: Uuid,
    cron: String,
    pool: sqlx::PgPool,
    queue: Arc<ApalisRedisQueue>,
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
