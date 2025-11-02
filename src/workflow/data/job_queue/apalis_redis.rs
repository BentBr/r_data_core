use super::JobQueue;
use crate::workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use async_trait::async_trait;

// Thin wrapper placeholder around Apalis Redis storage
pub struct ApalisRedisQueue;

impl ApalisRedisQueue {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl JobQueue for ApalisRedisQueue {
    async fn enqueue_fetch(&self, _job: FetchAndStageJob) -> anyhow::Result<()> {
        // TODO: integrate with apalis-redis storage and push job
        Ok(())
    }

    async fn enqueue_process(&self, _job: ProcessRawItemJob) -> anyhow::Result<()> {
        // TODO: integrate with apalis-redis storage and push job
        Ok(())
    }
}
