use env_logger;
use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core::config::AppConfig;
use r_data_core::workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core::workflow::data::job_queue::JobQueue;
use r_data_core::workflow::data::jobs::FetchAndStageJob;
use r_data_core::workflow::data::repository::WorkflowRepository;
use r_data_core::services::{WorkflowService, WorkflowRepositoryAdapter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Basic logger init
    let env = env_logger::Env::new().default_filter_or("info");
    env_logger::Builder::from_env(env)
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .format_module_path(true)
        .format_target(true)
        .init();

    info!("Starting data workflow worker");

    let config = AppConfig::from_env().map_err(|e| anyhow::anyhow!(e))?;

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await?;

    let _queue = ApalisRedisQueue::new();

    // Scheduler: scan workflows with cron and schedule tasks
    let scheduler = JobScheduler::new().await?;

    let repo = WorkflowRepository::new(pool.clone());
    let workflows = repo.list_scheduled_consumers().await?;

    for (workflow_id, cron) in workflows {
        let pool_clone = pool.clone();
        let job = Job::new_async(cron.as_str(), move |_uuid, _l| {
            let pool = pool_clone.clone();
            Box::pin(async move {
                info!("Enqueue fetch job for workflow {}", workflow_id);
                let trigger_id = Uuid::now_v7();
                let _ = ApalisRedisQueue::new()
                    .enqueue_fetch(FetchAndStageJob {
                        workflow_id,
                        trigger_id: Some(trigger_id),
                    })
                    .await;
                let repo = WorkflowRepository::new(pool.clone());
                if let Ok(run_uuid) = repo.insert_run_queued(workflow_id, trigger_id).await {
                    let _ = repo
                        .insert_run_log(
                            run_uuid,
                            "info",
                            "Run enqueued by scheduler",
                            Some(serde_json::json!({"trigger": trigger_id.to_string()})),
                        )
                        .await;
                }
            })
        })?;
        scheduler.add(job).await?;
    }

    scheduler.start().await?;
    info!("Worker scheduler started");

    // Simple processor loop: poll queued runs and mark them as processed.
    let repo_for_loop = WorkflowRepository::new(pool.clone());
    tokio::spawn(async move {
        let repo = repo_for_loop;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            if let Ok(run_ids) = repo.list_queued_runs(50).await {
                for run_id in run_ids {
                    let _ = repo.mark_run_running(run_id).await;
                    // Try to fetch & stage from workflow config if nothing staged yet
                    let staged_existing = repo.count_raw_items_for_run(run_id).await.unwrap_or(0);
                    if staged_existing == 0 {
                        // we need the workflow uuid for this run
                        if let Ok(Some(row)) = sqlx::query("SELECT workflow_uuid FROM workflow_runs WHERE uuid = $1")
                            .bind(run_id)
                            .fetch_optional(&pool)
                            .await
                        {
                            if let Ok(wf_uuid) = row.try_get::<uuid::Uuid, _>("workflow_uuid") {
                                // Use service to fetch & stage via adapters
                                let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
                                let service = WorkflowService::new(std::sync::Arc::new(adapter));
                                let _ = service.fetch_and_stage_from_config(wf_uuid, run_id).await;
                            }
                        }
                    }
                    // Process staged items with DSL
                    let adapter = r_data_core::services::WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
                    let service = r_data_core::services::WorkflowService::new(std::sync::Arc::new(adapter));
                    // get workflow uuid for run
                    let wf_uuid = sqlx::query("SELECT workflow_uuid FROM workflow_runs WHERE uuid = $1")
                        .bind(run_id)
                        .fetch_one(&pool)
                        .await
                        .ok()
                        .and_then(|row| row.try_get::<uuid::Uuid, _>("workflow_uuid").ok());
                    if let Some(wf_uuid) = wf_uuid {
                        match service.process_staged_items(wf_uuid, run_id).await {
                            Ok((processed, failed)) => {
                                let _ = repo.insert_run_log(run_id, "info", &format!("Run processed (processed_items={}, failed_items={})", processed, failed), None).await;
                                let _ = repo.mark_run_success(run_id, processed, failed).await;
                            }
                            Err(e) => {
                                let _ = repo.insert_run_log(run_id, "error", &format!("Run failed: {}", e), None).await;
                                let _ = repo.mark_run_failure(run_id, &format!("{}", e)).await;
                            }
                        }
                    } else {
                        let _ = repo.insert_run_log(run_id, "error", "Missing workflow_uuid for run", None).await;
                        let _ = repo.mark_run_failure(run_id, "Missing workflow_uuid").await;
                    }
                }
            }
        }
    });

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
