#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Test consumer loop helpers for workflow processing tests.

use r_data_core_core::cache::CacheManager;
use r_data_core_persistence::{
    DynamicEntityRepository, EntityDefinitionRepository, WorkflowRepository,
};
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{
    DynamicEntityService, EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

/// Configuration for spawning a consumer loop in tests
pub struct ConsumerLoopConfig {
    /// Database connection pool
    pub pool: PgPool,
    /// Redis queue for workflow jobs
    pub queue: ApalisRedisQueue,
    /// Cache manager for entity definitions
    pub cache_manager: Arc<CacheManager>,
    /// Queue fetch key (for logging)
    pub fetch_key: String,
}

/// Handle to a running consumer loop for testing
pub struct ConsumerLoopHandle {
    /// Sender to stop the consumer loop
    stop_tx: Option<oneshot::Sender<()>>,
    /// Join handle for the consumer loop task
    pub join_handle: tokio::task::JoinHandle<()>,
}

impl ConsumerLoopHandle {
    /// Stop the consumer loop
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Get or create the run UUID for a job
async fn get_or_create_run_uuid(
    repo: &WorkflowRepository,
    job: &FetchAndStageJob,
) -> Result<Uuid, String> {
    if let Some(run) = job.trigger_id {
        return Ok(run);
    }

    let external_trigger_id = Uuid::now_v7();
    repo.insert_run_queued(job.workflow_id, external_trigger_id)
        .await
        .map_err(|e| format!("Failed to create run for workflow {}: {e}", job.workflow_id))
}

/// Stage items for a workflow run if none exist
async fn stage_items_if_needed(pool: &PgPool, repo: &WorkflowRepository, run_uuid: Uuid) {
    let staged_existing = repo.count_raw_items_for_run(run_uuid).await.unwrap_or(0);
    if staged_existing == 0 {
        if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
            let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
            let service = WorkflowService::new(Arc::new(adapter));
            let _ = service.fetch_and_stage_from_config(wf_uuid, run_uuid).await;
        }
    }
}

/// Create a workflow service with entity support
fn create_workflow_service_with_entities(
    pool: &PgPool,
    cache_manager: &Arc<CacheManager>,
) -> WorkflowService {
    let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
    let de_repo = DynamicEntityRepository::new(pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new(Arc::new(ed_adapter), cache_manager.clone());
    let de_service = DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service))
}

/// Process a workflow run and log the results
async fn process_and_log_run(repo: &WorkflowRepository, service: &WorkflowService, run_uuid: Uuid) {
    let Some(wf_uuid) = repo
        .get_workflow_uuid_for_run(run_uuid)
        .await
        .ok()
        .flatten()
    else {
        log_run_error(repo, run_uuid, "Missing workflow_uuid for run").await;
        return;
    };

    match service.process_staged_items(wf_uuid, run_uuid).await {
        Ok((processed, failed)) => {
            let msg = format!("Run processed (processed_items={processed}, failed_items={failed})");
            let _ = repo.insert_run_log(run_uuid, "info", &msg, None).await;
            let _ = repo.mark_run_success(run_uuid, processed, failed).await;
        }
        Err(e) => {
            let msg = format!("Run failed: {e}");
            let _ = repo.insert_run_log(run_uuid, "error", &msg, None).await;
            let _ = repo.mark_run_failure(run_uuid, &msg).await;
        }
    }
}

/// Log an error for a run and mark it as failed
async fn log_run_error(repo: &WorkflowRepository, run_uuid: Uuid, message: &str) {
    let _ = repo.insert_run_log(run_uuid, "error", message, None).await;
    let _ = repo.mark_run_failure(run_uuid, message).await;
}

/// Process a single job from the queue
async fn process_job(job: FetchAndStageJob, pool: &PgPool, cache_manager: &Arc<CacheManager>) {
    log::info!(
        "Popped fetch job from queue: workflow_id={}, run_uuid={:?}",
        job.workflow_id,
        job.trigger_id
    );

    let repo = WorkflowRepository::new(pool.clone());

    let run_uuid = match get_or_create_run_uuid(&repo, &job).await {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("{e}");
            return;
        }
    };

    let _ = repo.mark_run_running(run_uuid).await;
    stage_items_if_needed(pool, &repo, run_uuid).await;

    let service = create_workflow_service_with_entities(pool, cache_manager);
    process_and_log_run(&repo, &service, run_uuid).await;
}

/// Spawn a consumer loop for testing purposes
///
/// This simulates the worker's consumer loop but with a stop signal
/// to allow tests to cleanly shut down the loop.
#[must_use]
pub fn spawn_test_consumer_loop(config: ConsumerLoopConfig) -> ConsumerLoopHandle {
    const MAX_BACKOFF_MS: u64 = 30_000;
    const BACKOFF_MULTIPLIER: u64 = 2;

    let (stop_tx, mut stop_rx) = oneshot::channel();
    let pool = config.pool;
    let queue = config.queue;
    let cache_manager = config.cache_manager;
    let fetch_key = config.fetch_key;

    let join_handle = tokio::spawn(async move {
        let mut retry_backoff_ms: u64 = 250;

        loop {
            if stop_rx.try_recv().is_ok() {
                log::info!("Consumer loop received stop signal, shutting down");
                break;
            }

            let pop_result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                queue.blocking_pop_fetch(),
            )
            .await;

            match pop_result {
                Ok(Ok(job)) => {
                    retry_backoff_ms = 250;
                    process_job(job, &pool, &cache_manager).await;
                }
                Ok(Err(e)) => {
                    log::error!(
                        "Queue pop failed from '{fetch_key}': {e}. Retrying after {retry_backoff_ms}ms..."
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(retry_backoff_ms)).await;
                    retry_backoff_ms = (retry_backoff_ms * BACKOFF_MULTIPLIER).min(MAX_BACKOFF_MS);
                }
                Err(_) => {
                    // Timeout - expected, loop continues to check stop signal
                }
            }
        }
    });

    ConsumerLoopHandle {
        stop_tx: Some(stop_tx),
        join_handle,
    }
}

/// Helper to create a test queue with unique keys
///
/// Returns `None` if `REDIS_URL` is not set.
pub async fn create_test_queue() -> Option<(ApalisRedisQueue, String, String)> {
    let url = std::env::var("REDIS_URL").ok()?;
    let test_id = Uuid::now_v7();
    let base_fetch =
        std::env::var("QUEUE_FETCH_KEY").unwrap_or_else(|_| "test_queue:fetch".to_string());
    let base_process =
        std::env::var("QUEUE_PROCESS_KEY").unwrap_or_else(|_| "test_queue:process".to_string());
    let fetch_key = format!("{base_fetch}:{test_id}");
    let process_key = format!("{base_process}:{test_id}");

    let queue = ApalisRedisQueue::from_parts(&url, &fetch_key, &process_key)
        .await
        .ok()?;
    Some((queue, fetch_key, process_key))
}
