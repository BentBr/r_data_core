use super::JobQueue;
use crate::workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use anyhow::Context;
use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, Client};

/// Redis-backed queue for workflow jobs.
/// Uses Redis Lists:
/// - RPUSH to enqueue
/// - BLPOP to consume (blocking)
pub struct ApalisRedisQueue {
    client: Option<Client>,
    fetch_queue_key: String,
    process_queue_key: String,
}

impl ApalisRedisQueue {
    /// Create a queue from a specific Redis URL and queue keys.
    /// Tests the connection immediately to fail fast if Redis is unreachable.
    pub async fn from_parts(url: &str, fetch_key: &str, process_key: &str) -> anyhow::Result<Self> {
        let client = Client::open(url).with_context(|| format!("invalid redis url: {}", url))?;

        // Test connection immediately
        let mut test_conn = client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get initial Redis connection for testing")?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut test_conn)
            .await
            .context("failed to ping Redis - connection test failed")?;

        log::info!(
            "Redis queue initialized: url={}, fetch_key={}, process_key={}",
            url,
            fetch_key,
            process_key
        );

        Ok(Self {
            client: Some(client),
            fetch_queue_key: fetch_key.to_string(),
            process_queue_key: process_key.to_string(),
        })
    }

    async fn get_conn(&self) -> anyhow::Result<MultiplexedConnection> {
        let client = self
            .client
            .as_ref()
            .context("Redis queue not configured (missing REDIS_URL)?")?;
        client
            .get_multiplexed_async_connection()
            .await
            .context("failed to get redis connection")
    }

    async fn push_json<T: serde::Serialize>(&self, key: &str, job: &T) -> anyhow::Result<()> {
        let mut conn = self.get_conn().await?;
        let payload = serde_json::to_string(job).context("failed to serialize job")?;
        log::info!("Enqueueing job to Redis key '{}': {}", key, payload);
        // RPUSH for queue semantics (append to tail)
        let result: i64 = redis::cmd("RPUSH")
            .arg(key)
            .arg(&payload)
            .query_async(&mut conn)
            .await
            .with_context(|| format!("failed to RPUSH job to Redis key '{}'", key))?;
        log::info!(
            "Successfully enqueued job: RPUSH returned length {} for key '{}'",
            result,
            key
        );
        Ok(())
    }

    /// Block until a fetch job is available, then return it.
    pub async fn blocking_pop_fetch(&self) -> anyhow::Result<FetchAndStageJob> {
        let mut conn = self.get_conn().await?;
        // BLPOP key 0 => block indefinitely
        // Returns VecBulkString [key, value]
        let result: Option<(String, String)> = redis::cmd("BLPOP")
            .arg(&self.fetch_queue_key)
            .arg(0)
            .query_async(&mut conn)
            .await
            .context("failed to BLPOP fetch queue from redis")?;
        if let Some((_key, value)) = result {
            let job: FetchAndStageJob =
                serde_json::from_str(&value).context("failed to deserialize fetch job")?;
            Ok(job)
        } else {
            // Should not happen with BLPOP 0, but handle defensively
            Err(anyhow::anyhow!("no job returned from BLPOP"))
        }
    }
}

#[async_trait]
impl JobQueue for ApalisRedisQueue {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> anyhow::Result<()> {
        self.push_json(&self.fetch_queue_key, &job).await
    }

    async fn enqueue_process(&self, job: ProcessRawItemJob) -> anyhow::Result<()> {
        self.push_json(&self.process_queue_key, &job).await
    }
}
