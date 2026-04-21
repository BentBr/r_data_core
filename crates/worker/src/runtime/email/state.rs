#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_services::MailService;
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

use super::EmailRuntime;

#[derive(Clone)]
pub(super) struct EmailConsumerState {
    pub(super) pool: sqlx::PgPool,
    pub(super) queue: Arc<ApalisRedisQueue>,
    pub(super) queue_email_key: String,
    pub(super) system_mail_service: Option<Arc<MailService>>,
    pub(super) workflow_mail_service: Option<Arc<MailService>>,
}

impl EmailConsumerState {
    pub(super) fn from_runtime(runtime: &EmailRuntime) -> Self {
        Self {
            pool: runtime.pool.clone(),
            queue: runtime.queue.clone(),
            queue_email_key: runtime.queue_email_key.clone(),
            system_mail_service: runtime.system_mail_service.clone(),
            workflow_mail_service: runtime.workflow_mail_service.clone(),
        }
    }
}
