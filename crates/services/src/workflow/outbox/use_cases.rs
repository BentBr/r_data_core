#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_persistence::{OutboxRepositoryTrait, WorkflowRepositoryTrait};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use uuid::Uuid;

use super::dispatch::WorkflowOutboxDispatcher;
use super::OutboxRetryPolicy;

/// Use case for enqueuing workflow fetch runs with optional outbox support.
pub struct EnqueueWorkflowFetchUseCase<'a> {
    repo: &'a Arc<dyn WorkflowRepositoryTrait>,
    queue: &'a dyn JobQueue,
    outbox_repository: Option<&'a dyn OutboxRepositoryTrait>,
    outbox_retry_policy: Option<&'a OutboxRetryPolicy>,
    use_outbox_for_fetch: bool,
}

impl<'a> EnqueueWorkflowFetchUseCase<'a> {
    #[must_use]
    pub const fn new(
        repo: &'a Arc<dyn WorkflowRepositoryTrait>,
        queue: &'a dyn JobQueue,
        outbox_repository: Option<&'a dyn OutboxRepositoryTrait>,
        outbox_retry_policy: Option<&'a OutboxRetryPolicy>,
        use_outbox_for_fetch: bool,
    ) -> Self {
        Self {
            repo,
            queue,
            outbox_repository,
            outbox_retry_policy,
            use_outbox_for_fetch,
        }
    }

    /// Enqueue a workflow run and schedule its fetch execution.
    ///
    /// # Errors
    /// Returns an error if the database or queue operation fails.
    pub async fn enqueue_run_for_fetch(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Option<Uuid>,
    ) -> r_data_core_core::error::Result<Uuid> {
        let trigger_id = trigger_id.unwrap_or_else(Uuid::now_v7);

        if self.use_outbox_for_fetch {
            if let Some(outbox_repo) = self.outbox_repository {
                let (run_uuid, outbox_uuid) = self
                    .repo
                    .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
                    .await?;
                WorkflowOutboxDispatcher::new(
                    Some(self.queue),
                    outbox_repo,
                    None,
                    None,
                    self.outbox_retry_policy,
                )
                .dispatch_fetch_run(workflow_uuid, run_uuid, outbox_uuid, 1)
                .await?;
                let _ = self
                    .repo
                    .insert_run_log(
                        run_uuid,
                        "info",
                        "Run enqueued",
                        Some(serde_json::json!({
                            "trigger": trigger_id.to_string(),
                            "outbox_uuid": outbox_uuid.to_string(),
                        })),
                    )
                    .await;
                return Ok(run_uuid);
            }
        }

        let run_uuid = self
            .repo
            .insert_run_queued(workflow_uuid, trigger_id)
            .await?;
        self.queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: workflow_uuid,
                trigger_id: Some(run_uuid),
            })
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({
                    "trigger": trigger_id.to_string(),
                })),
            )
            .await;
        Ok(run_uuid)
    }

    /// Dispatch the fetch job for an already created workflow run.
    ///
    /// # Errors
    /// Returns an error if dispatching fails.
    pub async fn dispatch_fetch_for_existing_run(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        if self.use_outbox_for_fetch {
            if let Some(outbox_repo) = self.outbox_repository {
                let outbox_uuid = outbox_repo
                    .insert_workflow_fetch_enqueue(workflow_uuid, run_uuid)
                    .await?;
                WorkflowOutboxDispatcher::new(
                    Some(self.queue),
                    outbox_repo,
                    None,
                    None,
                    self.outbox_retry_policy,
                )
                .dispatch_fetch_run(workflow_uuid, run_uuid, outbox_uuid, 1)
                .await?;
                let _ = self
                    .repo
                    .insert_run_log(
                        run_uuid,
                        "info",
                        "Run enqueued",
                        Some(serde_json::json!({
                            "run_uuid": run_uuid.to_string(),
                            "outbox_uuid": outbox_uuid.to_string(),
                        })),
                    )
                    .await;
                return Ok(());
            }
        }

        self.queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: workflow_uuid,
                trigger_id: Some(run_uuid),
            })
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({
                    "run_uuid": run_uuid.to_string(),
                })),
            )
            .await;
        Ok(())
    }
}

/// Use case for claiming and dispatching workflow outbox records in worker loops.
pub struct DispatchWorkflowOutboxBatchUseCase<'a> {
    queue: &'a dyn JobQueue,
    workflow_repository: &'a dyn WorkflowRepositoryTrait,
    outbox_repository: &'a dyn OutboxRepositoryTrait,
    worker_id: &'a str,
    batch_size: i64,
    stale_lease_secs: i64,
    outbox_retry_policy: Option<&'a OutboxRetryPolicy>,
}

impl<'a> DispatchWorkflowOutboxBatchUseCase<'a> {
    #[must_use]
    pub const fn new(
        queue: &'a dyn JobQueue,
        workflow_repository: &'a dyn WorkflowRepositoryTrait,
        outbox_repository: &'a dyn OutboxRepositoryTrait,
        worker_id: &'a str,
        batch_size: i64,
        stale_lease_secs: i64,
        outbox_retry_policy: Option<&'a OutboxRetryPolicy>,
    ) -> Self {
        Self {
            queue,
            workflow_repository,
            outbox_repository,
            worker_id,
            batch_size,
            stale_lease_secs,
            outbox_retry_policy,
        }
    }

    /// Run one claim-and-dispatch batch cycle.
    ///
    /// # Errors
    /// Returns an error if claiming or dispatching fails.
    pub async fn run_once(&self) -> r_data_core_core::error::Result<usize> {
        let stale_before =
            time::OffsetDateTime::now_utc() - time::Duration::seconds(self.stale_lease_secs);
        let _ = self
            .outbox_repository
            .requeue_stale_processing(stale_before)
            .await?;

        let records = self
            .outbox_repository
            .claim_due(self.batch_size, self.worker_id)
            .await?;
        let dispatcher = WorkflowOutboxDispatcher::new(
            Some(self.queue),
            self.outbox_repository,
            Some(self.workflow_repository),
            Some(self.worker_id),
            self.outbox_retry_policy,
        );
        let mut dispatched_count = 0usize;
        for record in records {
            dispatcher.dispatch_record(&record).await?;
            dispatched_count = dispatched_count.saturating_add(1);
        }

        Ok(dispatched_count)
    }
}
