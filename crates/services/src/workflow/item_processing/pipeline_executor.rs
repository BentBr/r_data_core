use super::status_handler::WorkflowItemStatusHandler;
use super::step_executor::WorkflowStepExecutor;
use super::WorkflowItemContext;
use crate::workflow::output_handling::WorkflowOutputDispatcher;
use r_data_core_workflow::dsl::{DslProgram, ToDef};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub struct WorkflowPipelineExecutor<'a> {
    program: &'a DslProgram,
    workflow_uuid: Uuid,
    run_uuid: Uuid,
    ctx: &'a WorkflowItemContext<'a>,
    fail_fast: bool,
}

impl<'a> WorkflowPipelineExecutor<'a> {
    #[must_use]
    pub const fn new(
        program: &'a DslProgram,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        ctx: &'a WorkflowItemContext<'a>,
        fail_fast: bool,
    ) -> Self {
        Self {
            program,
            workflow_uuid,
            run_uuid,
            ctx,
            fail_fast,
        }
    }

    /// Process one staged item including output side-effects and status updates.
    ///
    /// # Errors
    /// Returns an error if processing fails.
    pub async fn process_item(
        &self,
        payload: &JsonValue,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        match self.step_executor().execute(payload, item_uuid).await {
            Ok(outputs) => self.process_outputs(outputs, payload, item_uuid).await,
            Err(e) => {
                self.status_handler()
                    .handle_execution_error(e, item_uuid)
                    .await
            }
        }
    }

    /// Execute inline and return raw outputs without mutating item status.
    ///
    /// # Errors
    /// Returns an error if any pipeline step fails.
    pub async fn execute_inline(
        &self,
        payload: &JsonValue,
    ) -> r_data_core_core::error::Result<Vec<(ToDef, JsonValue)>> {
        self.step_executor()
            .execute(payload, Uuid::now_v7())
            .await
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|(_step_idx, to_def, produced)| (to_def, produced))
                    .collect()
            })
    }

    const fn step_executor(&self) -> WorkflowStepExecutor<'_> {
        WorkflowStepExecutor::new(self.program, self.run_uuid, self.ctx, self.fail_fast)
    }

    fn status_handler(&self) -> WorkflowItemStatusHandler<'_> {
        WorkflowItemStatusHandler::new(self.run_uuid, self.ctx.repo)
    }

    async fn process_outputs(
        &self,
        processed_outputs: Vec<(usize, ToDef, JsonValue)>,
        payload: &JsonValue,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        let output_dispatcher = WorkflowOutputDispatcher::new(self.ctx);
        for (step_index, to_def, produced) in processed_outputs {
            let push_ok = output_dispatcher
                .handle_format_push_output(
                    &to_def,
                    &produced,
                    self.workflow_uuid,
                    step_index,
                    item_uuid,
                    self.run_uuid,
                )
                .await?;
            if !push_ok {
                return self
                    .status_handler()
                    .mark_entity_operation_failed(item_uuid)
                    .await;
            }

            let email_ok = output_dispatcher
                .handle_email_output(&to_def, &produced, self.run_uuid)
                .await?;
            if !email_ok {
                return self
                    .status_handler()
                    .mark_entity_operation_failed(item_uuid)
                    .await;
            }

            let entity_ok = output_dispatcher
                .handle_entity_output(&to_def, &produced, payload, item_uuid, self.run_uuid)
                .await?;
            if !entity_ok {
                return self
                    .status_handler()
                    .mark_entity_operation_failed(item_uuid)
                    .await;
            }
        }

        self.status_handler().mark_item_processed(item_uuid).await
    }
}
