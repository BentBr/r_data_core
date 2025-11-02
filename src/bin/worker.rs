use env_logger;
use log::info;
use sqlx::postgres::PgPoolOptions;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core::config::AppConfig;
use r_data_core::workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core::workflow::data::job_queue::JobQueue;
use r_data_core::workflow::data::jobs::FetchAndStageJob;
use r_data_core::workflow::data::repository::WorkflowRepository;

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
                let _ = repo.insert_run_queued(workflow_id, trigger_id).await;
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
                    // TODO: execute FetchAndStage + ProcessRawItem jobs; for now, mark success
                    let _ = repo.mark_run_success(run_id, 0, 0).await;
                }
            }
        }
    });

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
