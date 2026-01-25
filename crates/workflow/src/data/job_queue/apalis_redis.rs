use super::JobQueue;
use crate::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use async_trait::async_trait;
use r_data_core_core::cache::test_redis_connection;
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
    ///
    /// # Errors
    /// Returns an error if the Redis connection cannot be established or the URL is invalid.
    pub async fn from_parts(
        url: &str,
        fetch_key: &str,
        process_key: &str,
    ) -> r_data_core_core::error::Result<Self> {
        let client = Client::open(url).map_err(|e| {
            r_data_core_core::error::Error::Config(format!("invalid redis url: {url}: {e}"))
        })?;

        // Test connection immediately
        let mut test_conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Cache(format!(
                    "failed to get initial Redis connection for testing: {e}"
                ))
            })?;
        test_redis_connection(&mut test_conn).await.map_err(|e| {
            r_data_core_core::error::Error::Cache(format!(
                "failed to ping Redis - connection test failed: {e}"
            ))
        })?;

        log::info!(
            "Redis queue initialized: url={url}, fetch_key={fetch_key}, process_key={process_key}"
        );

        Ok(Self {
            client: Some(client),
            fetch_queue_key: fetch_key.to_string(),
            process_queue_key: process_key.to_string(),
        })
    }

    async fn get_conn(&self) -> r_data_core_core::error::Result<MultiplexedConnection> {
        let client = self.client.as_ref().ok_or_else(|| {
            r_data_core_core::error::Error::Config(
                "Redis queue not configured (missing REDIS_URL)?".to_string(),
            )
        })?;
        client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Cache(format!(
                    "failed to get redis connection: {e}"
                ))
            })
    }

    #[allow(clippy::future_not_send)] // MultiplexedConnection is Send, this is a false positive
    async fn push_json<T: serde::Serialize>(
        &self,
        key: &str,
        job: &T,
    ) -> r_data_core_core::error::Result<()> {
        let mut conn = self.get_conn().await?;
        let payload = serde_json::to_string(job).map_err(|e| {
            r_data_core_core::error::Error::Deserialization(format!("failed to serialize job: {e}"))
        })?;
        log::info!("Enqueueing job to Redis key '{key}': {payload}");
        // RPUSH for queue semantics (append to tail)
        let result: i64 = redis::cmd("RPUSH")
            .arg(key)
            .arg(&payload)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Cache(format!(
                    "failed to RPUSH job to Redis key '{key}': {e}"
                ))
            })?;
        log::info!("Successfully enqueued job: RPUSH returned length {result} for key '{key}'");
        Ok(())
    }

    /// Block until a fetch job is available, then return it.
    ///
    /// # Errors
    /// Returns an error if the Redis connection fails or the job cannot be deserialized.
    pub async fn blocking_pop_fetch(&self) -> r_data_core_core::error::Result<FetchAndStageJob> {
        let mut conn = self.get_conn().await.map_err(|e| {
            log::error!(
                "Failed to get Redis connection for BLPOP on queue '{}': {e}",
                self.fetch_queue_key
            );
            e
        })?;
        // BLPOP key 0 => block indefinitely
        // Returns VecBulkString [key, value]
        let result: Option<(String, String)> = redis::cmd("BLPOP")
            .arg(&self.fetch_queue_key)
            .arg(0)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                log::error!("BLPOP failed on queue '{}': {e}", self.fetch_queue_key);
                r_data_core_core::error::Error::Cache(format!(
                    "failed to BLPOP fetch queue '{}' from redis: {e}",
                    self.fetch_queue_key
                ))
            })?;
        if let Some((_key, value)) = result {
            let job: FetchAndStageJob = serde_json::from_str(&value).map_err(|e| {
                log::error!(
                    "Failed to deserialize job from queue '{}': {e}. Raw value: {}",
                    self.fetch_queue_key,
                    if value.len() > 200 {
                        format!("{}... (truncated)", &value[..200])
                    } else {
                        value.clone()
                    }
                );
                r_data_core_core::error::Error::Deserialization(format!(
                    "failed to deserialize fetch job from queue '{}': {e}",
                    self.fetch_queue_key
                ))
            })?;
            Ok(job)
        } else {
            // Should not happen with BLPOP 0, but handle defensively
            log::warn!(
                "BLPOP returned None for queue '{}' (unexpected with timeout 0)",
                self.fetch_queue_key
            );
            Err(r_data_core_core::error::Error::Cache(format!(
                "no job returned from BLPOP on queue '{}'",
                self.fetch_queue_key
            )))
        }
    }
}

#[async_trait]
impl JobQueue for ApalisRedisQueue {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> r_data_core_core::error::Result<()> {
        self.push_json(&self.fetch_queue_key, &job).await
    }

    async fn enqueue_process(&self, job: ProcessRawItemJob) -> r_data_core_core::error::Result<()> {
        self.push_json(&self.process_queue_key, &job).await
    }
}
