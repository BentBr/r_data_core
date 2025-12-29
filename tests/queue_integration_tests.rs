#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob};
use uuid::Uuid;

async fn get_test_queue() -> Option<(ApalisRedisQueue, String, String)> {
    let url = std::env::var("REDIS_URL").ok()?;
    // Always use a unique queue key per test to ensure isolation
    // Append test ID even if env vars are set to prevent test interference
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

#[tokio::test]
async fn enqueue_and_pop_fetch_job_round_trip_if_redis_available() {
    let Some((queue, _fetch_key, _process_key)) = get_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };
    let wf = Uuid::now_v7();
    let run = Uuid::now_v7();

    // Enqueue
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf,
            trigger_id: Some(run),
        })
        .await
        .expect("enqueue fetch should succeed");

    // Pop
    let popped = queue
        .blocking_pop_fetch()
        .await
        .expect("pop should return previously enqueued job");

    assert_eq!(popped.workflow_id, wf);
    assert_eq!(popped.trigger_id, Some(run));
}

#[tokio::test]
async fn enqueue_process_job_if_redis_available() {
    let Some((queue, _fetch_key, process_key)) = get_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };
    let item = Uuid::now_v7();

    // Enqueue process job
    queue
        .enqueue_process(ProcessRawItemJob { raw_item_id: item })
        .await
        .expect("enqueue process should succeed");

    // Verify it was enqueued by checking Redis directly using the same process_key
    // (We can't pop process jobs with the current API, but we can verify they're in Redis)
    let url = std::env::var("REDIS_URL").expect("REDIS_URL should be set");

    let client = redis::Client::open(url).expect("Failed to create Redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to get Redis connection");

    let len: i64 = redis::cmd("LLEN")
        .arg(&process_key)
        .query_async(&mut conn)
        .await
        .expect("Failed to check queue length");

    assert!(len > 0, "Process queue should have at least one job");

    // Clean up: pop the job we just enqueued
    let _: Option<(String, String)> = redis::cmd("BLPOP")
        .arg(&process_key)
        .arg(1) // 1 second timeout
        .query_async(&mut conn)
        .await
        .expect("Failed to pop from process queue");
}

#[tokio::test]
async fn enqueue_multiple_jobs_fifo_ordering_if_redis_available() {
    let Some((queue, _fetch_key, _process_key)) = get_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };
    let wf = Uuid::now_v7();

    // Enqueue 3 jobs
    let runs = vec![Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
    for run in &runs {
        queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: wf,
                trigger_id: Some(*run),
            })
            .await
            .expect("enqueue should succeed");
    }

    // Pop and verify FIFO order
    for expected_run in &runs {
        let popped = queue
            .blocking_pop_fetch()
            .await
            .expect("pop should return job");
        assert_eq!(popped.workflow_id, wf);
        assert_eq!(popped.trigger_id, Some(*expected_run));
    }
}

#[tokio::test]
async fn queue_initialization_fails_with_invalid_redis_url() {
    let result =
        ApalisRedisQueue::from_parts("redis://invalid-host:9999", "test:fetch", "test:process")
            .await;

    assert!(result.is_err(), "Should fail with invalid Redis URL");
}

#[tokio::test]
async fn enqueue_fetch_without_trigger_id_if_redis_available() {
    let Some((queue, _fetch_key, _process_key)) = get_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };
    let wf = Uuid::now_v7();

    // Enqueue without trigger_id
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf,
            trigger_id: None,
        })
        .await
        .expect("enqueue fetch should succeed");

    // Pop
    let popped = queue
        .blocking_pop_fetch()
        .await
        .expect("pop should return previously enqueued job");

    assert_eq!(popped.workflow_id, wf);
    assert_eq!(popped.trigger_id, None);
}
