#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::time::Duration;

use log::{error, info};
use uuid::Uuid;

use r_data_core_persistence::WorkflowRepository;
use r_data_core_workflow::data::jobs::FetchAndStageJob;

use crate::runtime::WorkerRuntime;

use super::services::{build_fetch_service, build_processing_service};
use super::state::ConsumerState;

pub fn spawn_consumer_loop(runtime: &WorkerRuntime) {
    let state = ConsumerState::from_runtime(runtime);

    tokio::spawn(async move {
        info!(
            "Worker consumer loop started, waiting for jobs from queue '{}'...",
            state.queue_fetch_key
        );
        consume_job_loop(state).await;
    });
}

async fn consume_job_loop(state: ConsumerState) {
    const MAX_BACKOFF_MS: u64 = 30_000;
    const BACKOFF_MULTIPLIER: u64 = 2;

    let mut iteration_count: u64 = 0;
    let mut retry_backoff_ms: u64 = 250;

    loop {
        iteration_count = iteration_count.wrapping_add(1);
        if iteration_count.is_multiple_of(100) {
            info!(
                "Consumer loop alive, waiting for jobs from queue '{}' (iteration {iteration_count})",
                state.queue_fetch_key
            );
        }

        match state.queue.blocking_pop_fetch().await {
            Ok(job) => {
                retry_backoff_ms = 250;
                info!(
                    "Popped fetch job from queue: workflow_id={}, run_uuid={:?}",
                    job.workflow_id, job.trigger_id
                );
                handle_job(&state, job).await;
            }
            Err(e) => {
                error!(
                    "Queue pop failed from '{}': {e}. Retrying after {retry_backoff_ms}ms backoff...",
                    state.queue_fetch_key
                );
                tokio::time::sleep(Duration::from_millis(retry_backoff_ms)).await;
                retry_backoff_ms = (retry_backoff_ms * BACKOFF_MULTIPLIER).min(MAX_BACKOFF_MS);
            }
        }
    }
}

async fn handle_job(state: &ConsumerState, job: FetchAndStageJob) {
    let repo = WorkflowRepository::new(state.pool.clone());
    let run_uuid = if let Some(run) = job.trigger_id {
        run
    } else {
        let external_trigger_id = Uuid::now_v7();
        match repo
            .insert_run_queued(job.workflow_id, external_trigger_id)
            .await
        {
            Ok(uuid) => uuid,
            Err(e) => {
                error!(
                    "Failed to create run for workflow {}: {}",
                    job.workflow_id, e
                );
                return;
            }
        }
    };

    let _ = repo.mark_run_running(run_uuid).await;
    let staged_existing = repo.count_raw_items_for_run(run_uuid).await.unwrap_or(0);
    if staged_existing == 0 {
        if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
            let service = build_fetch_service(state);
            let _ = service.fetch_and_stage_from_config(wf_uuid, run_uuid).await;
        }
    }

    let service = build_processing_service(state);
    if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
        match service.process_staged_items(wf_uuid, run_uuid).await {
            Ok((processed, failed)) => {
                let _ = repo
                    .insert_run_log(
                        run_uuid,
                        "info",
                        &format!(
                            "Run processed (processed_items={processed}, failed_items={failed})"
                        ),
                        None,
                    )
                    .await;
                let _ = repo.mark_run_success(run_uuid, processed, failed).await;
            }
            Err(e) => {
                let _ = repo
                    .insert_run_log(run_uuid, "error", &format!("Run failed: {e}"), None)
                    .await;
                let _ = repo.mark_run_failure(run_uuid, &format!("{e}")).await;
            }
        }
    } else {
        let _ = repo
            .insert_run_log(run_uuid, "error", "Missing workflow_uuid for run", None)
            .await;
        let _ = repo
            .mark_run_failure(run_uuid, "Missing workflow_uuid")
            .await;
    }
}
