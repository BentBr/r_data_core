#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use std::sync::Arc;

/// Create a test queue client asynchronously
#[must_use]
pub async fn test_queue_client_async() -> Arc<ApalisRedisQueue> {
    // Use env if provided, otherwise fall back to localhost defaults.
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let fetch_key =
        std::env::var("QUEUE_FETCH_KEY").unwrap_or_else(|_| "queue:workflows:fetch".to_string());
    let process_key = std::env::var("QUEUE_PROCESS_KEY")
        .unwrap_or_else(|_| "queue:workflows:process".to_string());

    let queue = ApalisRedisQueue::from_parts(&url, &fetch_key, &process_key)
        .await
        .expect("Failed to construct test ApalisRedisQueue");

    Arc::new(queue)
}

