#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core_core::config::load_worker_config;
use r_data_core::services::adapters::EntityDefinitionRepositoryAdapter;
use r_data_core::services::bootstrap::{
    init_cache_manager, init_logger_with_default, init_pg_pool,
};
use r_data_core::services::{
    adapters::DynamicEntityRepositoryAdapter, worker::compute_reconcile_actions,
};
use r_data_core_services::{DynamicEntityService, EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_persistence::WorkflowRepository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Basic logger init
    init_logger_with_default("info");

    info!("Starting data workflow worker");

    let config = match load_worker_config() {
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

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await?;

    // Initialize cache manager (shares Redis with queue if available)
    let cache_manager =
        init_cache_manager(config.cache.clone(), Some(&config.queue.redis_url)).await;

    let queue_cfg = Arc::new(config.queue.clone());
    let _queue = ApalisRedisQueue::from_parts(
        &queue_cfg.redis_url,
        &queue_cfg.fetch_key,
        &queue_cfg.process_key,
    )
    .await?;

    // Scheduler: scan workflows with cron and schedule tasks
    let scheduler = JobScheduler::new().await?;

    let repo = WorkflowRepository::new(pool.clone());
    let scheduled: Arc<Mutex<HashMap<Uuid, (Uuid, String)>>> = Arc::new(Mutex::new(HashMap::new()));

    // function to create a job for a workflow and return (job_id, cron)
    let schedule_job = |scheduler: JobScheduler,
                        workflow_id: Uuid,
                        cron: String,
                        pool: sqlx::Pool<sqlx::Postgres>,
                        queue_cfg: Arc<r_data_core_core::config::QueueConfig>|
     -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<Uuid>> + Send>,
    > {
        Box::pin(async move {
            let pool_clone = pool.clone();
            let cron_clone = cron.clone();
            let job = Job::new_async(cron_clone.as_str(), move |_uuid, _l| {
                let pool = pool_clone.clone();
                let queue_cfg = queue_cfg.clone();
                Box::pin(async move {
                    info!(
                        "Schedule: creating run and enqueueing fetch job for workflow {}",
                        workflow_id
                    );
                    let repo = WorkflowRepository::new(pool.clone());
                    // Use a separate trigger identifier to store provenance; DB returns run_uuid
                    let external_trigger_id = Uuid::now_v7();
                    if let Ok(run_uuid) = repo
                        .insert_run_queued(workflow_id, external_trigger_id)
                        .await
                    {
                        // Enqueue fetch job with run_uuid in the trigger slot for downstream processing
                        match ApalisRedisQueue::from_parts(
                            &queue_cfg.redis_url,
                            &queue_cfg.fetch_key,
                            &queue_cfg.process_key,
                        )
                        .await
                        {
                            Ok(q) => {
                                match q
                                    .enqueue_fetch(FetchAndStageJob {
                                        workflow_id,
                                        trigger_id: Some(run_uuid),
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        info!("Successfully enqueued fetch job for workflow {} (run: {})", workflow_id, run_uuid);
                                    }
                                    Err(e) => {
                                        error!("Failed to enqueue fetch job for workflow {} (run: {}): {}", workflow_id, run_uuid, e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to create Redis queue client for workflow {}: {}",
                                    workflow_id, e
                                );
                            }
                        }
                        let _ = repo
                            .insert_run_log(
                                run_uuid,
                                "info",
                                "Run enqueued by scheduler",
                                Some(
                                    serde_json::json!({"trigger": external_trigger_id.to_string()}),
                                ),
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
            let job_id = schedule_job(
                scheduler.clone(),
                workflow_id,
                cron.clone(),
                pool.clone(),
                queue_cfg.clone(),
            )
            .await?;
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
        let queue_cfg_reconcile = queue_cfg.clone();
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
                            queue_cfg_reconcile.clone(),
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

    // Redis-backed consumer loop: block on queue and process runs.
    {
        let pool_for_consumer = pool.clone();
        let queue_cfg = queue_cfg.clone();
        let cache_manager_for_consumer = cache_manager.clone();
        tokio::spawn(async move {
            let queue = ApalisRedisQueue::from_parts(
                &queue_cfg.redis_url,
                &queue_cfg.fetch_key,
                &queue_cfg.process_key,
            )
            .await
            .expect("failed to init redis queue");
            loop {
                match queue.blocking_pop_fetch().await {
                    Ok(job) => {
                        let repo = WorkflowRepository::new(pool_for_consumer.clone());
                        // Determine or create the run UUID
                        let run_uuid = if let Some(run) = job.trigger_id {
                            run
                        } else {
                            // Create a run if not provided
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
                                    continue;
                                }
                            }
                        };
                        // Transition to running
                        let _ = repo.mark_run_running(run_uuid).await;
                        // If nothing staged yet, fetch & stage from config
                        let staged_existing =
                            repo.count_raw_items_for_run(run_uuid).await.unwrap_or(0);
                        if staged_existing == 0 {
                            if let Ok(Some(wf_uuid)) =
                                repo.get_workflow_uuid_for_run(run_uuid).await
                            {
                                let adapter = WorkflowRepositoryAdapter::new(
                                    WorkflowRepository::new(pool_for_consumer.clone()),
                                );
                                let service = WorkflowService::new(Arc::new(adapter));
                                let _ =
                                    service.fetch_and_stage_from_config(wf_uuid, run_uuid).await;
                            }
                        }
                        // Build services for processing
                        let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(
                            pool_for_consumer.clone(),
                        ));
                        let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool_for_consumer.clone());
                        let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
                        let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(pool_for_consumer.clone());
                        let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
                        // Use cache manager for entity definitions (shared with main API server via Redis)
                        let ed_service = EntityDefinitionService::new(
                            Arc::new(ed_adapter),
                            cache_manager_for_consumer.clone(),
                        );
                        let de_service =
                            DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
                        let service = WorkflowService::new_with_entities(
                            Arc::new(wf_adapter),
                            Arc::new(de_service),
                        );
                        // Process
                        if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
                            match service.process_staged_items(wf_uuid, run_uuid).await {
                                Ok((processed, failed)) => {
                                    let _ = repo
                                        .insert_run_log(
                                            run_uuid,
                                            "info",
                                            &format!(
                                                "Run processed (processed_items={}, failed_items={})",
                                                processed, failed
                                            ),
                                            None,
                                        )
                                        .await;
                                    let _ =
                                        repo.mark_run_success(run_uuid, processed, failed).await;
                                }
                                Err(e) => {
                                    let _ = repo
                                        .insert_run_log(
                                            run_uuid,
                                            "error",
                                            &format!("Run failed: {}", e),
                                            None,
                                        )
                                        .await;
                                    let _ =
                                        repo.mark_run_failure(run_uuid, &format!("{}", e)).await;
                                }
                            }
                        } else {
                            let _ = repo
                                .insert_run_log(
                                    run_uuid,
                                    "error",
                                    "Missing workflow_uuid for run",
                                    None,
                                )
                                .await;
                            let _ = repo
                                .mark_run_failure(run_uuid, "Missing workflow_uuid")
                                .await;
                        }
                    }
                    Err(e) => {
                        error!("Queue pop failed: {}", e);
                        // brief backoff before retrying to avoid hot loop
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                }
            }
        });
    }

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
