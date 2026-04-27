use crate::workflow::item_processing::WorkflowItemContext;
use crate::workflow::transform_execution::resolve_string_operands;
use r_data_core_workflow::dsl::ToDef;
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub(super) struct WorkflowEmailOutputHandler<'a> {
    ctx: &'a WorkflowItemContext<'a>,
}

impl<'a> WorkflowEmailOutputHandler<'a> {
    pub(super) const fn new(ctx: &'a WorkflowItemContext<'a>) -> Self {
        Self { ctx }
    }

    /// Handle Email to-target outputs.
    ///
    /// # Errors
    /// Returns an error if enqueueing fails fatally (non-fatal failures return `Ok(true)`).
    pub(super) async fn handle(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        let ToDef::Email {
            template_uuid,
            to,
            cc,
            mapping: _,
        } = to_def
        else {
            return Ok(true);
        };

        let (Some(_service), Some(queue)) = (self.ctx.mail.service, self.ctx.mail.queue) else {
            log::warn!("[workflow] Email output skipped: mail service or queue not configured");
            return Ok(true);
        };

        let to_addrs = resolve_string_operands(to, produced);
        let cc_addrs = cc.as_ref().map_or_else(Vec::new, |cc_list| {
            resolve_string_operands(cc_list, produced)
        });

        if to_addrs.is_empty() {
            log::warn!("[workflow] Email output skipped: no recipients resolved");
            return Ok(true);
        }

        let context_json = serde_json::to_string(produced).unwrap_or_default();

        let job = r_data_core_workflow::data::jobs::SendEmailJob {
            run_uuid: Some(run_uuid),
            to: to_addrs,
            cc: cc_addrs,
            subject: String::new(),
            body_text: String::new(),
            body_html: None,
            from_name_override: self.ctx.workflow_name.map(String::from),
            source: "workflow".to_string(),
            template_uuid: Some(uuid::Uuid::parse_str(template_uuid).unwrap_or_default()),
            template_context: Some(context_json),
        };

        if let Err(e) = queue.enqueue_email(job).await {
            log::error!("[workflow] Failed to enqueue email job for run {run_uuid}: {e}");
        }

        Ok(true)
    }
}
