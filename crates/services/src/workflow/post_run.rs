#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::SendEmailJob;
use r_data_core_workflow::dsl::on_complete::{OnComplete, PostRunAction, PostRunCondition};
use r_data_core_workflow::dsl::transform::StringOperand;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Aggregate run statistics available to post-run actions.
pub struct RunContext {
    pub run_uuid: Uuid,
    pub workflow_name: String,
    pub status: String,
    pub processed_items: i64,
    pub failed_items: i64,
    pub started_at: OffsetDateTime,
    pub finished_at: OffsetDateTime,
    pub error: Option<String>,
}

impl RunContext {
    /// Convert to a Handlebars-friendly JSON context.
    #[must_use]
    pub fn to_template_context(&self) -> Value {
        let total = self.processed_items + self.failed_items;
        let duration = (self.finished_at - self.started_at).whole_seconds();
        serde_json::json!({
            "run": {
                "uuid": self.run_uuid.to_string(),
                "workflow_name": self.workflow_name,
                "status": self.status,
                "processed_items": self.processed_items,
                "failed_items": self.failed_items,
                "total_items": total,
                "started_at": self.started_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                "finished_at": self.finished_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                "duration_seconds": duration,
                "error": self.error,
            }
        })
    }
}

const fn should_execute(condition: &PostRunCondition, context: &RunContext) -> bool {
    match condition {
        PostRunCondition::Always => true,
        PostRunCondition::OnSuccess => context.failed_items == 0,
        PostRunCondition::OnFailure => context.failed_items > 0,
    }
}

fn resolve_const_recipients(operands: &[StringOperand]) -> Vec<String> {
    operands
        .iter()
        .filter_map(|op| match op {
            StringOperand::ConstString { value } => Some(value.clone()),
            StringOperand::Field { field } => {
                log::warn!(
                    "Post-run email: field ref '{field}' used but no item context available -- skipping"
                );
                None
            }
        })
        .collect()
}

/// Execute post-run actions after all items are processed.
pub async fn execute_post_run_actions(
    on_complete: &OnComplete,
    context: &RunContext,
    mail_service: Option<&crate::mail::MailService>,
    queue: Option<&dyn JobQueue>,
) {
    let template_context = context.to_template_context();

    for (idx, action) in on_complete.actions.iter().enumerate() {
        let condition = match action {
            PostRunAction::SendEmail(e) => &e.condition,
        };

        if !should_execute(condition, context) {
            log::debug!(
                "[post-run] Skipping action {idx}: condition {condition:?} not met (status={})",
                context.status
            );
            continue;
        }

        match action {
            PostRunAction::SendEmail(email) => {
                handle_post_run_email(email, &template_context, context, mail_service, queue, idx)
                    .await;
            }
        }
    }
}

async fn handle_post_run_email(
    email: &r_data_core_workflow::dsl::on_complete::PostRunSendEmail,
    template_context: &Value,
    context: &RunContext,
    mail_service: Option<&crate::mail::MailService>,
    queue: Option<&dyn JobQueue>,
    idx: usize,
) {
    if mail_service.is_none() {
        log::warn!("[post-run] Action {idx}: mail service not configured, skipping");
        return;
    }
    let Some(queue) = queue else {
        log::warn!("[post-run] Action {idx}: queue not configured, skipping");
        return;
    };

    let to = resolve_const_recipients(&email.to);
    let cc = email
        .cc
        .as_ref()
        .map_or_else(Vec::new, |cc| resolve_const_recipients(cc));

    if to.is_empty() {
        log::warn!("[post-run] Action {idx}: no valid recipients, skipping");
        return;
    }

    let context_json = serde_json::to_string(template_context).unwrap_or_default();

    let job = SendEmailJob {
        run_uuid: Some(context.run_uuid),
        to,
        cc,
        subject: String::new(),
        body_text: context_json.clone(),
        body_html: None,
        from_name_override: Some(context.workflow_name.clone()),
        source: "workflow".to_string(),
        template_uuid: Some(uuid::Uuid::parse_str(&email.template_uuid).unwrap_or_default()),
        template_context: Some(context_json),
    };

    if let Err(e) = queue.enqueue_email(job).await {
        log::error!("[post-run] Action {idx}: failed to enqueue email: {e}");
    } else {
        log::info!(
            "[post-run] Action {idx}: email enqueued for run {}",
            context.run_uuid
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_execute_always() {
        let ctx = RunContext {
            run_uuid: Uuid::nil(),
            workflow_name: "test".to_string(),
            status: "success".to_string(),
            processed_items: 10,
            failed_items: 0,
            started_at: OffsetDateTime::now_utc(),
            finished_at: OffsetDateTime::now_utc(),
            error: None,
        };
        assert!(should_execute(&PostRunCondition::Always, &ctx));
    }

    #[test]
    fn should_execute_on_success_when_no_failures() {
        let ctx = RunContext {
            run_uuid: Uuid::nil(),
            workflow_name: "test".to_string(),
            status: "success".to_string(),
            processed_items: 10,
            failed_items: 0,
            started_at: OffsetDateTime::now_utc(),
            finished_at: OffsetDateTime::now_utc(),
            error: None,
        };
        assert!(should_execute(&PostRunCondition::OnSuccess, &ctx));
        assert!(!should_execute(&PostRunCondition::OnFailure, &ctx));
    }

    #[test]
    fn should_execute_on_failure_when_failures_exist() {
        let ctx = RunContext {
            run_uuid: Uuid::nil(),
            workflow_name: "test".to_string(),
            status: "partial_failure".to_string(),
            processed_items: 8,
            failed_items: 2,
            started_at: OffsetDateTime::now_utc(),
            finished_at: OffsetDateTime::now_utc(),
            error: None,
        };
        assert!(!should_execute(&PostRunCondition::OnSuccess, &ctx));
        assert!(should_execute(&PostRunCondition::OnFailure, &ctx));
    }

    #[test]
    fn resolve_const_recipients_filters_fields() {
        let operands = vec![
            StringOperand::ConstString {
                value: "a@b.com".to_string(),
            },
            StringOperand::Field {
                field: "user.email".to_string(),
            },
            StringOperand::ConstString {
                value: "c@d.com".to_string(),
            },
        ];
        let result = resolve_const_recipients(&operands);
        assert_eq!(result, vec!["a@b.com", "c@d.com"]);
    }

    #[test]
    fn template_context_contains_run_fields() {
        let ctx = RunContext {
            run_uuid: Uuid::nil(),
            workflow_name: "Import Customers".to_string(),
            status: "success".to_string(),
            processed_items: 42,
            failed_items: 3,
            started_at: OffsetDateTime::now_utc(),
            finished_at: OffsetDateTime::now_utc(),
            error: None,
        };
        let tc = ctx.to_template_context();
        let run = &tc["run"];
        assert_eq!(run["workflow_name"], "Import Customers");
        assert_eq!(run["processed_items"], 42);
        assert_eq!(run["failed_items"], 3);
        assert_eq!(run["total_items"], 45);
    }
}
