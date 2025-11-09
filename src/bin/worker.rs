use log::{debug, error, info};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core::config::WorkerConfig;
use r_data_core::services::adapters::EntityDefinitionRepositoryAdapter;
use r_data_core::services::{
    worker::compute_reconcile_actions, DynamicEntityRepositoryAdapter, EntityDefinitionService,
    WorkflowRepositoryAdapter, WorkflowService,
};
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

    let config = match WorkerConfig::from_env() {
        Ok(cfg) => {
            debug!("Loaded conf: {:?}", cfg);
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            panic!("Failed to load configuration: {}", e);
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await?;

    let _queue = ApalisRedisQueue::new();

    // Scheduler: scan workflows with cron and schedule tasks
    let scheduler = JobScheduler::new().await?;

    let repo = WorkflowRepository::new(pool.clone());
    let scheduled: Arc<Mutex<HashMap<Uuid, (Uuid, String)>>> = Arc::new(Mutex::new(HashMap::new()));

    // function to create a job for a workflow and return (job_id, cron)
    let schedule_job = |scheduler: JobScheduler,
                        workflow_id: Uuid,
                        cron: String,
                        pool: sqlx::Pool<sqlx::Postgres>|
     -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<Uuid>> + Send>,
    > {
        Box::pin(async move {
            let pool_clone = pool.clone();
            let cron_clone = cron.clone();
            let job = Job::new_async(cron_clone.as_str(), move |_uuid, _l| {
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
            let job_id = scheduler.add(job).await?;
            Ok(job_id)
        })
    };

    // initial load
    {
        let workflows = repo.list_scheduled_consumers().await?;
        for (workflow_id, cron) in workflows {
            let job_id =
                schedule_job(scheduler.clone(), workflow_id, cron.clone(), pool.clone()).await?;
            scheduled.lock().await.insert(workflow_id, (job_id, cron));
        }
    }

    scheduler.start().await?;
    info!("Worker scheduler started");

    // Reconcile scheduler with DB periodically (detect enabled/disabled/cron changes)
    {
        let scheduler_clone = scheduler.clone();
        let repo_clone = WorkflowRepository::new(pool.clone());
        let pool_clone2 = pool.clone();
        let scheduled_map = scheduled.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                config.job_queue_update_interval_secs,
            ));
            loop {
                interval.tick().await;
                if let Ok(db_workflows) = repo_clone.list_scheduled_consumers().await {
                    let mut map = scheduled_map.lock().await;
                    // Current set from DB
                    let mut current_set: HashMap<Uuid, String> = HashMap::new();
                    for (wf_id, cron) in db_workflows.iter() {
                        current_set.insert(*wf_id, cron.clone());
                    }
                    // Build existing set wf_id -> cron
                    let mut existing_set: HashMap<Uuid, String> = HashMap::new();
                    for (wf_id, (_job_id, cron)) in map.iter() {
                        existing_set.insert(*wf_id, cron.clone());
                    }
                    let (wf_to_remove, wf_to_add) =
                        compute_reconcile_actions(&existing_set, &current_set);
                    // Remove jobs for wf_to_remove
                    for wf_id in wf_to_remove {
                        if let Some((job_id, _)) = map.get(&wf_id) {
                            let _ = scheduler_clone.remove(job_id).await;
                        }
                        map.remove(&wf_id);
                    }
                    // Add or update jobs
                    for (wf_id, cron) in wf_to_add {
                        if let Ok(job_id) = schedule_job(
                            scheduler_clone.clone(),
                            wf_id,
                            cron.clone(),
                            pool_clone2.clone(),
                        )
                        .await
                        {
                            map.insert(wf_id, (job_id, cron));
                        }
                    }
                }
            }
        });
    }

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
                        if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_id).await {
                            // Use service to fetch & stage via adapters
                            let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(
                                pool.clone(),
                            ));
                            let service = WorkflowService::new(std::sync::Arc::new(adapter));
                            let _ = service.fetch_and_stage_from_config(wf_uuid, run_id).await;
                        }
                    }
                    // Process staged items with DSL (with entity persistence)
                    let wf_adapter = r_data_core::services::WorkflowRepositoryAdapter::new(
                        WorkflowRepository::new(pool.clone()),
                    );
                    // Build DynamicEntity service
                    let de_repo = r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository::new(pool.clone());
                    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
                    let ed_repo = r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository::new(pool.clone());
                    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
                    let ed_service = EntityDefinitionService::new(std::sync::Arc::new(ed_adapter));
                    let de_service = r_data_core::services::DynamicEntityService::new(
                        std::sync::Arc::new(de_adapter),
                        std::sync::Arc::new(ed_service),
                    );
                    let service = r_data_core::services::WorkflowService::new_with_entities(
                        std::sync::Arc::new(wf_adapter),
                        std::sync::Arc::new(de_service),
                    );
                    // get workflow uuid for run using repository
                    if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_id).await {
                        match service.process_staged_items(wf_uuid, run_id).await {
                            Ok((processed, failed)) => {
                                let _ = repo
                                    .insert_run_log(
                                        run_id,
                                        "info",
                                        &format!(
                                            "Run processed (processed_items={}, failed_items={})",
                                            processed, failed
                                        ),
                                        None,
                                    )
                                    .await;
                                let _ = repo.mark_run_success(run_id, processed, failed).await;
                            }
                            Err(e) => {
                                let _ = repo
                                    .insert_run_log(
                                        run_id,
                                        "error",
                                        &format!("Run failed: {}", e),
                                        None,
                                    )
                                    .await;
                                let _ = repo.mark_run_failure(run_id, &format!("{}", e)).await;
                            }
                        }
                    } else {
                        let _ = repo
                            .insert_run_log(run_id, "error", "Missing workflow_uuid for run", None)
                            .await;
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
