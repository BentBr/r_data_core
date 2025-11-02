use crate::workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use async_trait::async_trait;

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> anyhow::Result<()>;
    async fn enqueue_process(&self, job: ProcessRawItemJob) -> anyhow::Result<()>;
}

pub mod apalis_redis;
