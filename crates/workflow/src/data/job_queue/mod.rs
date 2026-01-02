use crate::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use async_trait::async_trait;

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> r_data_core_core::error::Result<()>;
    async fn enqueue_process(&self, job: ProcessRawItemJob) -> r_data_core_core::error::Result<()>;
}

pub mod apalis_redis;
