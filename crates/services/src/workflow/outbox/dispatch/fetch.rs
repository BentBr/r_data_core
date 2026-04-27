use uuid::Uuid;

use r_data_core_workflow::data::jobs::FetchAndStageJob;

use super::super::policy::{workflow_outbox_retry_at, OutboxRetryPolicy};
use super::super::support::is_permanent_outbox_failure;
use super::super::WORKFLOW_OUTBOX_MAX_ATTEMPTS;
use super::dispatcher::WorkflowOutboxDispatcher;

impl WorkflowOutboxDispatcher<'_> {
    /// Dispatch a fetch run to queue or transition to dead-letter/retry.
    ///
    /// # Errors
    /// Returns an error if queue interaction or state transition fails.
    pub async fn dispatch_fetch_run(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        outbox_uuid: Uuid,
        attempt_count: i32,
    ) -> r_data_core_core::error::Result<()> {
        let Some(queue) = self.queue else {
            self.outbox_repo
                .mark_dead_letter(
                    outbox_uuid,
                    "Workflow fetch outbox requires a queue",
                    self.locked_by,
                )
                .await?;
            return Ok(());
        };

        let job = FetchAndStageJob {
            workflow_id: workflow_uuid,
            trigger_id: Some(run_uuid),
        };

        match queue.enqueue_fetch(job).await {
            Ok(()) => {
                self.outbox_repo
                    .mark_delivered(outbox_uuid, self.locked_by)
                    .await?;
            }
            Err(e) => {
                if is_permanent_outbox_failure(&e) {
                    self.outbox_repo
                        .mark_dead_letter(outbox_uuid, &e.to_string(), self.locked_by)
                        .await?;
                    return Ok(());
                }

                let default_policy = OutboxRetryPolicy::default();
                let policy = self.retry_policy.map_or(&default_policy, |policy| policy);
                if attempt_count >= WORKFLOW_OUTBOX_MAX_ATTEMPTS {
                    self.outbox_repo
                        .mark_dead_letter(outbox_uuid, &e.to_string(), self.locked_by)
                        .await?;
                } else {
                    let next_available_at = workflow_outbox_retry_at(attempt_count, policy);
                    self.outbox_repo
                        .mark_retry(
                            outbox_uuid,
                            &e.to_string(),
                            next_available_at,
                            self.locked_by,
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}
