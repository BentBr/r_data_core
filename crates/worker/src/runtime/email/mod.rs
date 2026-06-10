#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use log::warn;
use r_data_core_core::config::WorkerConfig;
use r_data_core_services::MailService;
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

mod handler;
mod runner;
mod state;

pub(crate) use runner::spawn_email_consumer_loop;

#[derive(Clone)]
pub(crate) struct EmailRuntime {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) queue: Arc<ApalisRedisQueue>,
    pub(crate) queue_email_key: String,
    pub(crate) system_mail_service: Option<Arc<MailService>>,
    pub(crate) workflow_mail_service: Option<Arc<MailService>>,
}

impl EmailRuntime {
    pub(crate) fn workflow_mail_service(&self) -> Option<Arc<MailService>> {
        self.workflow_mail_service.clone()
    }
}

pub(crate) fn bootstrap_email_runtime(
    config: &WorkerConfig,
    pool: sqlx::PgPool,
    queue: Arc<ApalisRedisQueue>,
) -> EmailRuntime {
    let system_mail_service =
        config
            .mail
            .system
            .as_ref()
            .and_then(|smtp| match MailService::new(smtp) {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    warn!("System mail service not available: {e}");
                    None
                }
            });
    let workflow_mail_service =
        config
            .mail
            .workflow
            .as_ref()
            .and_then(|smtp| match MailService::new(smtp) {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    warn!("Workflow mail service not available: {e}");
                    None
                }
            });

    EmailRuntime {
        pool,
        queue,
        queue_email_key: config.queue.email_key.clone(),
        system_mail_service,
        workflow_mail_service,
    }
}
