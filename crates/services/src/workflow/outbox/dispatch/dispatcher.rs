use uuid::Uuid;

use r_data_core_core::outbox::{
    OutboxMessage, WORKFLOW_FETCH_ENQUEUE_KIND, WORKFLOW_FETCH_TOPIC, WORKFLOW_PUSH_ENQUEUE_KIND,
    WORKFLOW_PUSH_TOPIC,
};
use r_data_core_persistence::{OutboxRepositoryTrait, WorkflowRepositoryTrait};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;

use super::super::policy::OutboxRetryPolicy;

/// Dispatcher component responsible for outbox record delivery and state transitions.
pub struct WorkflowOutboxDispatcher<'a> {
    pub(super) queue: Option<&'a dyn JobQueue>,
    pub(super) outbox_repo: &'a dyn OutboxRepositoryTrait,
    pub(super) workflow_repo: Option<&'a dyn WorkflowRepositoryTrait>,
    pub(super) locked_by: Option<&'a str>,
    pub(super) retry_policy: Option<&'a OutboxRetryPolicy>,
}

impl<'a> WorkflowOutboxDispatcher<'a> {
    #[must_use]
    pub const fn new(
        queue: Option<&'a dyn JobQueue>,
        outbox_repo: &'a dyn OutboxRepositoryTrait,
        workflow_repo: Option<&'a dyn WorkflowRepositoryTrait>,
        locked_by: Option<&'a str>,
        retry_policy: Option<&'a OutboxRetryPolicy>,
    ) -> Self {
        Self {
            queue,
            outbox_repo,
            workflow_repo,
            locked_by,
            retry_policy,
        }
    }

    /// Dispatch any supported outbox record type.
    ///
    /// # Errors
    /// Returns an error if the underlying queue/push or database operation fails.
    pub async fn dispatch_record(
        &self,
        record: &OutboxMessage,
    ) -> r_data_core_core::error::Result<()> {
        let locked_by = self.locked_by.or(record.locked_by.as_deref());

        if record.topic == WORKFLOW_FETCH_TOPIC && record.kind == WORKFLOW_FETCH_ENQUEUE_KIND {
            let job: FetchAndStageJob = match serde_json::from_value(record.payload.clone()) {
                Ok(job) => job,
                Err(e) => {
                    self.outbox_repo
                        .mark_dead_letter(
                            record.uuid,
                            &format!("Invalid workflow outbox payload: {e}"),
                            locked_by,
                        )
                        .await?;
                    return Ok(());
                }
            };

            let Some(run_uuid) = job.trigger_id else {
                self.outbox_repo
                    .mark_dead_letter(
                        record.uuid,
                        "Missing trigger_id in workflow outbox payload",
                        locked_by,
                    )
                    .await?;
                return Ok(());
            };

            return WorkflowOutboxDispatcher::new(
                self.queue,
                self.outbox_repo,
                self.workflow_repo,
                locked_by,
                self.retry_policy,
            )
            .dispatch_fetch_run(
                job.workflow_id,
                run_uuid,
                record.uuid,
                record.attempt_count.saturating_add(1),
            )
            .await;
        }

        if record.topic == WORKFLOW_PUSH_TOPIC && record.kind == WORKFLOW_PUSH_ENQUEUE_KIND {
            return self.dispatch_push_record(record).await;
        }

        self.outbox_repo
            .mark_dead_letter(record.uuid, "Unsupported outbox message type", locked_by)
            .await?;
        Ok(())
    }

    pub(super) async fn mark_dead_letter_for_record(
        &self,
        record_uuid: Uuid,
        message: &str,
        locked_by: Option<&str>,
    ) -> r_data_core_core::error::Result<()> {
        self.outbox_repo
            .mark_dead_letter(record_uuid, message, locked_by)
            .await
    }
}
