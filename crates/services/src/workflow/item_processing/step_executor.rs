use super::WorkflowItemContext;
use crate::workflow::transform_execution::execute_async_transform;
use r_data_core_workflow::dsl::{DslProgram, ToDef, Transform};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub(super) struct WorkflowStepExecutor<'a> {
    program: &'a DslProgram,
    run_uuid: Uuid,
    ctx: &'a WorkflowItemContext<'a>,
    fail_fast: bool,
}

impl<'a> WorkflowStepExecutor<'a> {
    pub(super) const fn new(
        program: &'a DslProgram,
        run_uuid: Uuid,
        ctx: &'a WorkflowItemContext<'a>,
        fail_fast: bool,
    ) -> Self {
        Self {
            program,
            run_uuid,
            ctx,
            fail_fast,
        }
    }

    /// Execute workflow steps one at a time, running async transforms between steps.
    ///
    /// # Errors
    /// Returns an error if step preparation/finalization fails or `fail_fast` is enabled for
    /// async-transform failures.
    pub(super) async fn execute(
        &self,
        payload: &JsonValue,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Vec<(usize, ToDef, JsonValue)>> {
        let mut results: Vec<(usize, ToDef, JsonValue)> = Vec::new();
        let mut previous_step_output: Option<JsonValue> = None;

        for step_idx in 0..self.program.steps.len() {
            let (mut normalized, transform) =
                self.program
                    .prepare_step(step_idx, payload, previous_step_output.as_ref())?;

            if Self::is_async_transform(transform) {
                if let Some(de_service) = self.ctx.dynamic_entity_service {
                    if let Err(e) = execute_async_transform(
                        transform,
                        &mut normalized,
                        de_service,
                        self.run_uuid,
                        self.ctx.jwt,
                        self.ctx.mail,
                    )
                    .await
                    {
                        self.log_async_transform_error(&e, item_uuid, step_idx, transform)
                            .await;
                        if self.fail_fast {
                            return Err(e);
                        }
                    }
                }
            }

            if matches!(transform, Transform::BuildPath(_)) {
                DslProgram::apply_build_path(step_idx, transform, &mut normalized)?;
            }

            let (to_def, produced) = self.program.finalize_step(step_idx, &normalized)?;
            previous_step_output = Some(self.program.get_next_step_input(
                step_idx,
                &normalized,
                &produced,
            )?);

            results.push((step_idx, to_def, produced));
        }

        Ok(results)
    }

    const fn is_async_transform(transform: &Transform) -> bool {
        matches!(
            transform,
            Transform::ResolveEntityPath(_)
                | Transform::GetOrCreateEntity(_)
                | Transform::Authenticate(_)
                | Transform::SendEmail(_)
        )
    }

    async fn log_async_transform_error(
        &self,
        error: &r_data_core_core::error::Error,
        item_uuid: Uuid,
        step_idx: usize,
        transform: &Transform,
    ) {
        let error_msg = error.to_string();
        log::error!(
            "[workflow] Async transform failed for item {item_uuid} at step {step_idx}: {error_msg}"
        );

        if let Err(log_err) = self
            .ctx
            .repo
            .insert_run_log(
                self.run_uuid,
                "error",
                &format!("Step {step_idx}: Async transform failed"),
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "step_idx": step_idx,
                    "transform_type": format!("{:?}", transform),
                    "error": error_msg
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }
    }
}
