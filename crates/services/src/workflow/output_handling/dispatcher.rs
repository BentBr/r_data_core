use super::email_handler::WorkflowEmailOutputHandler;
use super::entity_handler::WorkflowEntityOutputHandler;
use super::push_handler::WorkflowPushOutputHandler;
use crate::workflow::item_processing::WorkflowItemContext;
use r_data_core_workflow::dsl::ToDef;
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub struct WorkflowOutputDispatcher<'a> {
    ctx: &'a WorkflowItemContext<'a>,
}

impl<'a> WorkflowOutputDispatcher<'a> {
    #[must_use]
    pub const fn new(ctx: &'a WorkflowItemContext<'a>) -> Self {
        Self { ctx }
    }

    /// Handle Format outputs with Push mode.
    ///
    /// # Errors
    /// Returns an error if serialization, authentication, or push fails.
    pub async fn handle_format_push_output(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        workflow_uuid: Uuid,
        step_index: usize,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        WorkflowPushOutputHandler::new(self.ctx)
            .handle(
                to_def,
                produced,
                workflow_uuid,
                step_index,
                item_uuid,
                run_uuid,
            )
            .await
    }

    /// Handle Email outputs.
    ///
    /// # Errors
    /// Returns an error if enqueueing fails fatally (non-fatal failures return `Ok(true)`).
    pub async fn handle_email_output(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        WorkflowEmailOutputHandler::new(self.ctx)
            .handle(to_def, produced, run_uuid)
            .await
    }

    /// Handle Entity outputs.
    ///
    /// # Errors
    /// Returns an error if entity creation/update fails.
    pub async fn handle_entity_output(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        payload: &JsonValue,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        WorkflowEntityOutputHandler::new(self.ctx)
            .handle(to_def, produced, payload, item_uuid, run_uuid)
            .await
    }
}
