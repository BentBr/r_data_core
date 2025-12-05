#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use sqlx::PgPool;
use std::sync::Arc;

/// Create a test queue client asynchronously
///
/// # Panics
/// Panics if `ApalisRedisQueue::from_parts` fails
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

/// Create a test queue client synchronously (for blocking contexts)
///
/// # Panics
/// Panics if runtime creation or queue construction fails
#[must_use]
pub fn test_queue_client() -> Arc<ApalisRedisQueue> {
    // For sync contexts, use spawn_blocking to avoid blocking the async runtime
    // This is safe because we're creating a new runtime in the blocking thread
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let fetch_key =
        std::env::var("QUEUE_FETCH_KEY").unwrap_or_else(|_| "queue:workflows:fetch".to_string());
    let process_key = std::env::var("QUEUE_PROCESS_KEY")
        .unwrap_or_else(|_| "queue:workflows:process".to_string());

    // Use spawn_blocking to run async code in a blocking context
    // This avoids deadlocks when called from within a tokio runtime
    let queue = if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're in a tokio runtime, use spawn_blocking
        handle
            .block_on(tokio::task::spawn_blocking(move || {
                // Create a new runtime in the blocking thread
                tokio::runtime::Runtime::new()
                    .expect("Failed to create runtime for test_queue_client")
                    .block_on(ApalisRedisQueue::from_parts(&url, &fetch_key, &process_key))
            }))
            .expect("Failed to spawn blocking task")
            .expect("Failed to join blocking task")
    } else {
        // Not in a runtime, create one
        tokio::runtime::Runtime::new()
            .expect("Failed to create runtime for test_queue_client")
            .block_on(ApalisRedisQueue::from_parts(&url, &fetch_key, &process_key))
            .expect("Failed to construct queue")
    };

    Arc::new(queue)
}

/// Create a workflow service for testing
#[must_use]
pub fn make_workflow_service(pool: &PgPool) -> WorkflowService {
    let repo = WorkflowRepository::new(pool.clone());
    let adapter = WorkflowRepositoryAdapter::new(repo);
    WorkflowService::new(Arc::new(adapter))
}
