#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_services::workflow::outbox::OutboxRetryPolicy;
use r_data_core_services::MailService;
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

use crate::runtime::WorkerRuntime;

#[derive(Clone)]
pub(super) struct ConsumerState {
    pub(super) pool: sqlx::PgPool,
    pub(super) queue: Arc<ApalisRedisQueue>,
    pub(super) queue_fetch_key: String,
    pub(super) cache_manager: Arc<r_data_core_core::cache::CacheManager>,
    pub(super) outbox_repo: Option<Arc<r_data_core_persistence::OutboxRepository>>,
    pub(super) outbox_retry_policy: Option<OutboxRetryPolicy>,
    pub(super) jwt_secret: Option<String>,
    pub(super) jwt_expiration: u64,
    pub(super) outbox_fetch_enabled_default: bool,
    pub(super) outbox_push_enabled_default: bool,
    pub(super) workflow_mail_service: Option<Arc<MailService>>,
}

impl ConsumerState {
    pub(super) fn from_runtime(
        runtime: &WorkerRuntime,
        workflow_mail_service: Option<Arc<MailService>>,
    ) -> Self {
        Self {
            pool: runtime.pool.clone(),
            queue: runtime.queue.clone(),
            queue_fetch_key: runtime.queue_fetch_key.clone(),
            cache_manager: runtime.cache_manager.clone(),
            outbox_repo: runtime.outbox_repo.clone(),
            outbox_retry_policy: runtime.outbox_retry_policy,
            jwt_secret: runtime.jwt_secret.clone(),
            jwt_expiration: runtime.jwt_expiration,
            outbox_fetch_enabled_default: runtime.outbox_fetch_enabled_default,
            outbox_push_enabled_default: runtime.outbox_push_enabled_default,
            workflow_mail_service,
        }
    }
}
