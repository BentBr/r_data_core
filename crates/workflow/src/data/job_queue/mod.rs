use crate::data::jobs::{FetchAndStageJob, ProcessRawItemJob, SendEmailJob};
use async_trait::async_trait;

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> r_data_core_core::error::Result<()>;
    async fn enqueue_process(&self, job: ProcessRawItemJob) -> r_data_core_core::error::Result<()>;
    /// Enqueue an email sending job
    async fn enqueue_email(&self, job: SendEmailJob) -> r_data_core_core::error::Result<()>;
    /// Pop an email job (blocks until one is available)
    async fn blocking_pop_email(&self) -> r_data_core_core::error::Result<SendEmailJob>;
}

pub mod apalis_redis;
