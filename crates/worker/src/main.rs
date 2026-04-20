#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use r_data_core_core::config::load_worker_config;
use r_data_core_core::outbox::WORKFLOW_OUTBOX_NOTIFY_CHANNEL;
use r_data_core_persistence::{ComponentVersionRepository, WorkflowRepository};

/// Current version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");
use r_data_core_persistence::OutboxRepository;
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_services::compute_reconcile_actions;
use r_data_core_services::workflow::outbox::{
    claim_and_dispatch_workflow_outbox_with_stale_lease, OutboxRetryPolicy,
};
use r_data_core_services::LicenseService;
use r_data_core_services::{
    DynamicEntityService, EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use sqlx::postgres::PgListener;
use tokio::sync::Notify;

#[tokio::main]
#[allow(clippy::too_many_lines)] // Main function orchestrates many components
async fn main() -> r_data_core_core::error::Result<()> {
    // Basic logger init
    init_logger_with_default("info");

    info!("Starting data workflow worker");

    let config = match load_worker_config() {
        Ok(cfg) => {
            debug!("Loaded conf: {cfg:?}");
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {e}");
            panic!("Failed to load configuration: {e}");
        }
    };

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to initialize database pool: {e}"))
    })?;

    // Initialize cache manager (shares Redis with queue if available)
    let cache_manager =
        init_cache_manager(config.cache.clone(), Some(&config.queue.redis_url)).await;

    // Verify license on startup
    let license_service = LicenseService::new(config.license.clone(), cache_manager.clone());
    license_service.verify_license_on_startup("worker").await;

    // Report worker version to database
    let version_repo = ComponentVersionRepository::new(pool.clone());
    if let Err(e) = version_repo.upsert("worker", VERSION).await {
        warn!("Failed to report worker version: {e}");
    } else {
        info!("Worker version {VERSION} registered");
    }

    let queue_cfg = Arc::new(config.queue.clone());
    let queue = Arc::new(
        ApalisRedisQueue::from_parts(
            &queue_cfg.redis_url,
            &queue_cfg.fetch_key,
            &queue_cfg.process_key,
        )
        .await?,
    );
    let outbox_repo = if config.outbox_enabled {
        Some(Arc::new(OutboxRepository::new(pool.clone())))
    } else {
        None
    };
    let outbox_db_url = config.database.connection_string.clone();
    let outbox_retry_policy = if config.outbox_enabled {
        Some(OutboxRetryPolicy::new(
            config.outbox_retry_base_delay_secs,
            config.outbox_retry_multiplier,
            config.outbox_retry_max_delay_secs,
        ))
    } else {
        None
    };

    // Scheduler: scan workflows with cron and schedule tasks
    let scheduler = JobScheduler::new().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to create job scheduler: {e}"))
    })?;

    let repo = WorkflowRepository::new(pool.clone());
    let scheduled_workflows: Arc<Mutex<HashMap<Uuid, (Uuid, String)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // function to create a job for a workflow and return (job_id, cron)
    let schedule_job = move |scheduler: JobScheduler,
                             workflow_id: Uuid,
                             cron: String,
                             pool: sqlx::Pool<sqlx::Postgres>,
                             queue: Arc<ApalisRedisQueue>,
                             outbox_repo: Option<Arc<OutboxRepository>>|
          -> std::pin::Pin<
        Box<dyn std::future::Future<Output = r_data_core_core::error::Result<Uuid>> + Send>,
    > {
        Box::pin(async move {
            let pool_clone = pool.clone();
            let cron_clone = cron.clone();
            let outbox_repo = outbox_repo.clone();
            let job = Job::new_async(cron_clone.as_str(), move |_uuid, _l| {
                let pool = pool_clone.clone();
                let queue = queue.clone();
                let outbox_repo = outbox_repo.clone();
                Box::pin(async move {
                    info!(
                        "Schedule: creating run and enqueueing fetch job for workflow {workflow_id}"
                    );
                    let external_trigger_id = Uuid::now_v7();
                    let workflow_service = {
                        let base = WorkflowService::new(Arc::new(WorkflowRepositoryAdapter::new(
                            WorkflowRepository::new(pool.clone()),
                        )));
                        if let Some(outbox_repo) = outbox_repo.clone() {
                            let base = base.with_outbox_repository(outbox_repo);
                            if let Some(policy) = outbox_retry_policy {
                                base.with_outbox_retry_policy(policy)
                            } else {
                                base
                            }
                        } else {
                            base
                        }
                    };
                    let _ = workflow_service
                        .enqueue_run_for_fetch(
                            workflow_id,
                            queue.as_ref(),
                            Some(external_trigger_id),
                        )
                        .await;
                })
            })
            .map_err(|e| {
                r_data_core_core::error::Error::Config(format!("Failed to create job: {e}"))
            })?;
            let job_id = scheduler.add(job).await.map_err(|e| {
                r_data_core_core::error::Error::Config(format!(
                    "Failed to add job to scheduler: {e}"
                ))
            })?;
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
                queue.clone(),
                outbox_repo.clone(),
            )
            .await?;
            scheduled_workflows
                .lock()
                .await
                .insert(workflow_id, (job_id, cron));
        }
    }

    scheduler.start().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to start scheduler: {e}"))
    })?;
    info!("Worker scheduler started");

    // Reconcile scheduler with DB periodically (detect enabled/disabled/cron changes)
    {
        let scheduler_clone = scheduler.clone();
        let repo_clone = WorkflowRepository::new(pool.clone());
        let pool_clone2 = pool.clone();
        let scheduled_map = scheduled_workflows.clone();
        let queue_for_reconcile = queue.clone();
        let outbox_repo_for_reconcile = outbox_repo.clone();
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
                    for (wf_id, cron) in &db_workflows {
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
                            queue_for_reconcile.clone(),
                            outbox_repo_for_reconcile.clone(),
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
        const MAX_BACKOFF_MS: u64 = 30_000; // Max 30 seconds
        const BACKOFF_MULTIPLIER: u64 = 2;

        let pool_for_consumer = pool.clone();
        let queue_cfg = queue_cfg.clone();
        let queue_for_consumer = queue.clone();
        let outbox_repo_for_consumer = outbox_repo.clone();
        let cache_manager_for_consumer = cache_manager.clone();
        let jwt_secret_for_consumer: Option<String> = std::env::var("JWT_SECRET").ok();
        let jwt_expiration_for_consumer: u64 = std::env::var("JWT_EXPIRATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(86_400);
        tokio::spawn(async move {
            info!(
                "Worker consumer loop started, waiting for jobs from queue '{}'...",
                queue_cfg.fetch_key
            );

            let mut iteration_count: u64 = 0;
            let mut retry_backoff_ms: u64 = 250; // Start with 250ms backoff

            loop {
                iteration_count = iteration_count.wrapping_add(1);
                // Log health check every 100 iterations (roughly every few minutes if no jobs)
                if iteration_count.is_multiple_of(100) {
                    info!(
                        "Consumer loop alive, waiting for jobs from queue '{}' (iteration {})",
                        queue_cfg.fetch_key, iteration_count
                    );
                }

                match queue_for_consumer.blocking_pop_fetch().await {
                    Ok(job) => {
                        // Reset backoff on successful pop
                        retry_backoff_ms = 250;
                        info!(
                            "Popped fetch job from queue: workflow_id={}, run_uuid={:?}",
                            job.workflow_id, job.trigger_id
                        );
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
                                let mut service = WorkflowService::new(Arc::new(adapter));
                                if let Some(outbox_repo) = outbox_repo_for_consumer.clone() {
                                    service = service.with_outbox_repository(outbox_repo);
                                    if let Some(policy) = outbox_retry_policy {
                                        service = service.with_outbox_retry_policy(policy);
                                    }
                                }
                                let _ =
                                    service.fetch_and_stage_from_config(wf_uuid, run_uuid).await;
                            }
                        }
                        // Build services for processing
                        let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(
                            pool_for_consumer.clone(),
                        ));
                        let de_repo = r_data_core_persistence::DynamicEntityRepository::new(
                            pool_for_consumer.clone(),
                        );
                        let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
                        let ed_repo = r_data_core_persistence::EntityDefinitionRepository::new(
                            pool_for_consumer.clone(),
                        );
                        let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
                        // Use cache manager for entity definitions (shared with main API server via Redis)
                        let ed_service = EntityDefinitionService::new(
                            Arc::new(ed_adapter),
                            cache_manager_for_consumer.clone(),
                        );
                        let de_service =
                            DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
                        let mut service = WorkflowService::new_with_entities(
                            Arc::new(wf_adapter),
                            Arc::new(de_service),
                        )
                        .with_jwt_config(
                            jwt_secret_for_consumer.clone(),
                            jwt_expiration_for_consumer,
                        );
                        if let Some(outbox_repo) = outbox_repo_for_consumer.clone() {
                            service = service.with_outbox_repository(outbox_repo);
                            if let Some(policy) = outbox_retry_policy {
                                service = service.with_outbox_retry_policy(policy);
                            }
                        }
                        // Process
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
                                    let _ =
                                        repo.mark_run_success(run_uuid, processed, failed).await;
                                }
                                Err(e) => {
                                    let _ = repo
                                        .insert_run_log(
                                            run_uuid,
                                            "error",
                                            &format!("Run failed: {e}"),
                                            None,
                                        )
                                        .await;
                                    let _ = repo.mark_run_failure(run_uuid, &format!("{e}")).await;
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
                        error!(
                            "Queue pop failed from '{}': {e}. Retrying after {}ms backoff...",
                            queue_cfg.fetch_key, retry_backoff_ms
                        );
                        // Exponential backoff with max cap to avoid hot loop and reduce Redis load
                        tokio::time::sleep(std::time::Duration::from_millis(retry_backoff_ms))
                            .await;
                        // Increase backoff for next retry, but cap at MAX_BACKOFF_MS
                        retry_backoff_ms =
                            (retry_backoff_ms * BACKOFF_MULTIPLIER).min(MAX_BACKOFF_MS);
                    }
                }
            }
        });
    }

    // Workflow outbox recovery loop: claim pending messages and dispatch them to Redis.
    if let Some(outbox_repo_for_outbox) = outbox_repo.clone() {
        let queue_for_outbox = queue.clone();
        let outbox_notify = Arc::new(Notify::new());

        {
            let outbox_db_url = outbox_db_url.clone();
            let outbox_notify = outbox_notify.clone();
            tokio::spawn(async move {
                match PgListener::connect(&outbox_db_url).await {
                    Ok(mut listener) => {
                        if let Err(e) = listener.listen(WORKFLOW_OUTBOX_NOTIFY_CHANNEL).await {
                            error!(
                                "Failed to listen for workflow outbox notifications on '{WORKFLOW_OUTBOX_NOTIFY_CHANNEL}': {e}"
                            );
                            return;
                        }

                        loop {
                            match listener.recv().await {
                                Ok(_notification) => {
                                    outbox_notify.notify_one();
                                }
                                Err(e) => {
                                    error!(
                                        "Workflow outbox notification listener failed: {e}; retrying"
                                    );
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to initialize workflow outbox notification listener: {e}");
                    }
                }
            });
        }

        tokio::spawn(async move {
            const OUTBOX_BATCH_SIZE: i64 = 50;
            let worker_id = format!("workflow-outbox-{}", Uuid::now_v7());
            let poll_interval =
                Duration::from_secs(std::cmp::max(5, config.job_queue_update_interval_secs));
            let mut sleep_until = Instant::now();

            loop {
                tokio::select! {
                    () = tokio::time::sleep_until(tokio::time::Instant::from_std(sleep_until)) => {}
                    () = outbox_notify.notified() => {}
                }

                let mut dispatched_total = 0usize;
                loop {
                    match claim_and_dispatch_workflow_outbox_with_stale_lease(
                        queue_for_outbox.as_ref(),
                        outbox_repo_for_outbox.as_ref(),
                        &worker_id,
                        OUTBOX_BATCH_SIZE,
                        config.outbox_stale_lease_secs,
                        outbox_retry_policy.as_ref(),
                    )
                    .await
                    {
                        Ok(dispatched) => {
                            if dispatched == 0 {
                                break;
                            }
                            dispatched_total = dispatched_total.saturating_add(dispatched);
                        }
                        Err(e) => {
                            error!("Workflow outbox dispatcher failed: {e}");
                            break;
                        }
                    }
                }

                if dispatched_total > 0 {
                    info!(
                        "Dispatched {dispatched_total} workflow outbox message(s) via worker outbox loop"
                    );
                }

                let next_available_at = match outbox_repo_for_outbox.next_available_at().await {
                    Ok(value) => value,
                    Err(e) => {
                        error!("Failed to query next workflow outbox availability: {e}");
                        None
                    }
                };

                let now = Instant::now();
                let fallback = now.checked_add(poll_interval).unwrap_or(now);
                sleep_until = next_available_at
                    .and_then(|value| {
                        let now_utc = OffsetDateTime::now_utc();
                        if value <= now_utc {
                            Some(now)
                        } else {
                            let delta = value - now_utc;
                            let secs = u64::try_from(delta.whole_seconds()).unwrap_or(0);
                            now.checked_add(Duration::from_secs(secs))
                        }
                    })
                    .unwrap_or(fallback);
            }
        });
    }

    // Park forever
    futures::future::pending::<()>().await;
    Ok(())
}
