#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use log::{error, info};
use r_data_core_core::system_log::SystemLogStatus;
use r_data_core_persistence::{
    EmailTemplateRepository, EmailTemplateRepositoryTrait, SystemLogRepository,
};
use r_data_core_services::SystemLogService;
use r_data_core_workflow::data::jobs::SendEmailJob;

use super::state::EmailConsumerState;

pub(super) async fn handle_email_job(state: &EmailConsumerState, job: SendEmailJob) {
    let mail_service = match job.source.as_str() {
        "system" => state.system_mail_service.as_ref(),
        "workflow" => state.workflow_mail_service.as_ref(),
        _ => {
            error!("Unknown email job source: {}", job.source);
            return;
        }
    };

    let Some(mail_service) = mail_service else {
        error!(
            "Mail service not configured for source '{}', dropping email job",
            job.source
        );
        log_email_result(
            &state.pool,
            &job,
            SystemLogStatus::Failed,
            &format!(
                "Email to {:?} dropped: {} mail not configured",
                job.to, job.source
            ),
            Some(serde_json::json!({
                "to": job.to.clone(),
                "subject": job.subject.clone(),
                "source": job.source.clone(),
            })),
        )
        .await;
        return;
    };

    let (subject, body_text, body_html) = if let (Some(template_uuid), Some(context_json)) =
        (job.template_uuid, &job.template_context)
    {
        let template_repo = EmailTemplateRepository::new(state.pool.clone());
        match template_repo.get_by_uuid(template_uuid).await {
            Ok(Some(template)) => {
                let context: serde_json::Value =
                    serde_json::from_str(context_json).unwrap_or_default();
                let subject = mail_service
                    .render_template(&template.subject_template, &context)
                    .unwrap_or_else(|_| template.subject_template.clone());
                let body_text = mail_service
                    .render_template(&template.body_text_template, &context)
                    .unwrap_or_else(|_| template.body_text_template.clone());
                let body_html = mail_service
                    .render_template(&template.body_html_template, &context)
                    .ok();
                (subject, body_text, body_html)
            }
            Ok(None) => {
                error!("Failed to load email template {template_uuid}");
                return;
            }
            Err(e) => {
                error!("Failed to load email template {template_uuid}: {e}");
                return;
            }
        }
    } else {
        (
            job.subject.clone(),
            job.body_text.clone(),
            job.body_html.clone(),
        )
    };

    match mail_service
        .send_raw_email(&job.to, &subject, &body_text, body_html.as_deref())
        .await
    {
        Ok(()) => {
            info!("Email sent successfully to {:?}", job.to);
            log_email_result(
                &state.pool,
                &job,
                SystemLogStatus::Success,
                &format!("Email sent to {}", job.to.join(", ")),
                Some(serde_json::json!({
                    "to": job.to.clone(),
                    "cc": job.cc.clone(),
                    "subject": subject,
                    "body_html": body_html,
                    "source": job.source.clone(),
                })),
            )
            .await;
        }
        Err(e) => {
            error!("Failed to send email to {:?}: {e}", job.to);
            log_email_result(
                &state.pool,
                &job,
                SystemLogStatus::Failed,
                &format!("Failed to send email to {}: {e}", job.to.join(", ")),
                Some(serde_json::json!({
                    "to": job.to.clone(),
                    "subject": subject,
                    "source": job.source.clone(),
                    "error": e.to_string(),
                })),
            )
            .await;
        }
    }
}

async fn log_email_result(
    pool: &sqlx::PgPool,
    job: &SendEmailJob,
    status: SystemLogStatus,
    summary: &str,
    details: Option<serde_json::Value>,
) {
    let service = SystemLogService::new(Arc::new(SystemLogRepository::new(pool.clone())));
    service
        .log_email_sent(None, job.template_uuid, summary, details, status)
        .await;
}
