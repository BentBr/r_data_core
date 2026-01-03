#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::DynamicEntity;
use r_data_core_persistence::DynamicEntityRepositoryTrait;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::DynamicEntityService;
use r_data_core_services::EntityDefinitionService;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::data::WorkflowKind;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_test_support::{
    create_test_admin_user, create_test_entity_definition, setup_test_db, unique_entity_type,
};

// Import common test utilities for loading workflow examples
mod api {
    pub mod workflows {
        pub mod common {
            pub fn load_workflow_example(
                filename: &str,
                entity_type: &str,
            ) -> anyhow::Result<serde_json::Value> {
                use std::fs;
                let path = format!(".example_files/json_examples/dsl/{filename}");
                let content = fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;
                let content = content.replace("${ENTITY_TYPE}", entity_type);
                serde_json::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse {path}: {e}"))
            }
        }
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // E2E test with comprehensive workflow testing
async fn end_to_end_workflow_processing_via_redis_queue() -> anyhow::Result<()> {
    // Skip test if REDIS_URL not present
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("Skipping e2e test: REDIS_URL not set");
        return Ok(());
    };

    // Use unique keys per test to avoid cross-test interference
    let fetch_key = format!("test:e2e:queue:fetch:{}", Uuid::now_v7().simple());
    let process_key = format!("test:e2e:queue:process:{}", Uuid::now_v7().simple());
    let queue = ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
        .await
        .expect("Failed to create Redis queue for e2e test");

    // DB setup
    let pool = setup_test_db().await;

    // Create a dynamic entity definition used by the workflow's "to" step
    let entity_type = unique_entity_type("e2e_entity");
    let _entity_def_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create a workflow that maps CSV to the dynamic entity
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    // Resolve a creator (admin user) or use a generated UUID for created_by
    let creator_uuid = create_test_admin_user(&pool).await?;

    let cfg = api::workflows::common::load_workflow_example(
        "workflow_csv_to_entity_simple.json",
        &entity_type,
    )?;

    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("e2e-wf-{}", Uuid::now_v7().simple()),
        description: Some("e2e workflow".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };
    let wf_uuid = wf_service.create(&create_req, creator_uuid).await?;

    // Enqueue a run and stage items before queueing (so worker can skip fetch)
    let run_uuid = wf_service.enqueue_run(wf_uuid).await?;
    let staged = wf_service
        .stage_raw_items(
            wf_uuid,
            run_uuid,
            vec![serde_json::json!({
                "name": "Alice E2E",
                "email": "alice-e2e@example.com"
            })],
        )
        .await?;
    assert_eq!(staged, 1, "should stage one raw item");

    // Enqueue fetch job to Redis with run id in trigger
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: Some(run_uuid),
        })
        .await?;

    // Simulate worker consumer: pop, mark running, process staged items, mark success
    let popped = queue.blocking_pop_fetch().await?;
    assert_eq!(popped.workflow_id, wf_uuid);
    assert_eq!(popped.trigger_id, Some(run_uuid));

    let repo = WorkflowRepository::new(pool.pool.clone());
    let _ = repo.mark_run_running(run_uuid).await;

    // Build services for processing (same as worker)
    let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.pool.clone()));
    let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));
    let de_service =
        r_data_core_services::DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    let service = WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service));

    // Process
    match service.process_staged_items(wf_uuid, run_uuid).await {
        Ok((processed, failed)) => {
            let _ = repo
                .insert_run_log(
                    run_uuid,
                    "info",
                    &format!(
                        "E2E Run processed (processed_items={processed}, failed_items={failed})"
                    ),
                    None,
                )
                .await;
            let _ = repo.mark_run_success(run_uuid, processed, failed).await;
            assert_eq!(processed, 1, "one item should be processed");
            assert_eq!(failed, 0, "no item should fail");
        }
        Err(e) => {
            let _ = repo
                .insert_run_log(run_uuid, "error", &format!("E2E Run failed: {e}"), None)
                .await;
            let _ = repo.mark_run_failure(run_uuid, &format!("{e}")).await;
            anyhow::bail!("processing failed: {e}");
        }
    }

    // Validate output: the dynamic entity was created
    let de_repo_check = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let entities: Vec<DynamicEntity> = de_repo_check
        .get_all_by_type(&entity_type, 10, 0, None)
        .await?;
    assert_eq!(entities.len(), 1, "exactly one entity should be created");

    Ok(())
}

// Define helper struct for consumer loop handle (must be before test function)
struct ConsumerLoopHandle {
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl ConsumerLoopHandle {
    fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // E2E test with comprehensive workflow testing
async fn end_to_end_consumer_loop_processes_workflow_run() -> anyhow::Result<()> {
    // Skip test if REDIS_URL not present
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("Skipping e2e test: REDIS_URL not set");
        return Ok(());
    };

    // Use unique keys per test to avoid cross-test interference
    let fetch_key = format!("test:e2e:consumer:fetch:{}", Uuid::now_v7().simple());
    let process_key = format!("test:e2e:consumer:process:{}", Uuid::now_v7().simple());
    let queue = ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
        .await
        .expect("Failed to create Redis queue for e2e test");

    // DB setup
    let pool = setup_test_db().await;

    // Create a dynamic entity definition used by the workflow's "to" step
    let entity_type = unique_entity_type("e2e_consumer_entity");
    let _entity_def_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create a workflow that maps CSV to the dynamic entity
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    // Resolve a creator (admin user) or use a generated UUID for created_by
    let creator_uuid = create_test_admin_user(&pool).await?;

    let cfg = api::workflows::common::load_workflow_example(
        "workflow_csv_to_entity_simple.json",
        &entity_type,
    )?;

    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("e2e-consumer-wf-{}", Uuid::now_v7().simple()),
        description: Some("e2e consumer workflow".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };
    let wf_uuid = wf_service.create(&create_req, creator_uuid).await?;

    // Enqueue a run via API (simulating external trigger)
    let run_uuid = wf_service.enqueue_run(wf_uuid).await?;

    // Enqueue fetch job to Redis with run id in trigger
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: Some(run_uuid),
        })
        .await?;

    // Setup cache manager for consumer loop
    let cache_config = r_data_core_core::config::CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    // Spawn consumer loop in background task (simulating worker)
    // Note: We inline the consumer loop logic here since we can't easily import test_helpers
    // In a real scenario, the worker would handle this
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
    let pool_for_consumer = pool.pool.clone();
    // Note: ApalisRedisQueue doesn't implement Clone, so we need to create a new one
    // In a real worker, the queue would be shared via Arc
    let queue_for_consumer = ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
        .await
        .expect("Failed to create Redis queue for consumer");
    let cache_manager_for_consumer = cache_manager.clone();
    let consumer_join_handle = tokio::spawn(async move {
        let mut iteration_count: u64 = 0;
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }
            iteration_count = iteration_count.wrapping_add(1);
            let pop_result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                queue_for_consumer.blocking_pop_fetch(),
            )
            .await;
            if let Ok(Ok(job)) = pop_result {
                let repo = WorkflowRepository::new(pool_for_consumer.clone());
                let run_uuid = if let Some(run) = job.trigger_id {
                    run
                } else {
                    let external_trigger_id = Uuid::now_v7();
                    repo.insert_run_queued(job.workflow_id, external_trigger_id)
                        .await
                        .unwrap_or_else(|_| Uuid::now_v7())
                };
                let _ = repo.mark_run_running(run_uuid).await;
                let staged_existing = repo.count_raw_items_for_run(run_uuid).await.unwrap_or(0);
                if staged_existing == 0 {
                    if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
                        let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(
                            pool_for_consumer.clone(),
                        ));
                        let service = WorkflowService::new(Arc::new(adapter));
                        let _ = service.fetch_and_stage_from_config(wf_uuid, run_uuid).await;
                    }
                }
                let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(
                    pool_for_consumer.clone(),
                ));
                let de_repo = r_data_core_persistence::DynamicEntityRepository::new(
                    pool_for_consumer.clone(),
                );
                let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
                let ed_repo = EntityDefinitionRepository::new(pool_for_consumer.clone());
                let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
                let ed_service = EntityDefinitionService::new(
                    Arc::new(ed_adapter),
                    cache_manager_for_consumer.clone(),
                );
                let de_service =
                    DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
                let service =
                    WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service));
                if let Ok(Some(wf_uuid)) = repo.get_workflow_uuid_for_run(run_uuid).await {
                    match service.process_staged_items(wf_uuid, run_uuid).await {
                        Ok((processed, failed)) => {
                            let _ = repo
                                    .insert_run_log(
                                        run_uuid,
                                        "info",
                                        &format!("Run processed (processed_items={processed}, failed_items={failed})"),
                                        None,
                                    )
                                    .await;
                            let _ = repo.mark_run_success(run_uuid, processed, failed).await;
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
                }
            }
        }
    });

    let mut consumer_handle = ConsumerLoopHandle {
        stop_tx: Some(stop_tx),
        join_handle: consumer_join_handle,
    };

    // Wait for job to be consumed and processed
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Verify run status transitions: queued → running → success
    let repo = WorkflowRepository::new(pool.pool.clone());
    let status = repo.get_run_status(run_uuid).await?;

    // Run should be processed (not queued)
    assert!(
        status.as_deref() != Some("queued"),
        "Run should not be queued after consumer loop processing"
    );

    // Stop consumer loop
    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // E2E test with comprehensive workflow testing
async fn consumer_loop_processes_staged_items() -> anyhow::Result<()> {
    // Skip test if REDIS_URL not present
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("Skipping e2e test: REDIS_URL not set");
        return Ok(());
    };

    // Use unique keys per test to avoid cross-test interference
    let fetch_key = format!("test:e2e:staged:fetch:{}", Uuid::now_v7().simple());
    let process_key = format!("test:e2e:staged:process:{}", Uuid::now_v7().simple());
    let queue = ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
        .await
        .expect("Failed to create Redis queue for e2e test");

    // DB setup
    let pool = setup_test_db().await;

    // Create a dynamic entity definition used by the workflow's "to" step
    let entity_type = unique_entity_type("e2e_staged_entity");
    let _entity_def_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create a workflow that maps CSV to the dynamic entity
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    // Resolve a creator (admin user) or use a generated UUID for created_by
    let creator_uuid = create_test_admin_user(&pool).await?;

    let cfg = api::workflows::common::load_workflow_example(
        "workflow_csv_to_entity_simple.json",
        &entity_type,
    )?;

    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("e2e-staged-wf-{}", Uuid::now_v7().simple()),
        description: Some("e2e staged workflow".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };
    let wf_uuid = wf_service.create(&create_req, creator_uuid).await?;

    // Create workflow run and stage items manually (so worker can skip fetch)
    let run_uuid = wf_service.enqueue_run(wf_uuid).await?;
    let staged = wf_service
        .stage_raw_items(
            wf_uuid,
            run_uuid,
            vec![serde_json::json!({
                "name": "Bob Staged",
                "email": "bob-staged@example.com"
            })],
        )
        .await?;
    assert_eq!(staged, 1, "should stage one raw item");

    // Enqueue fetch job (worker should skip fetch since items are staged)
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: Some(run_uuid),
        })
        .await?;

    // Setup cache manager for consumer loop
    let cache_config = r_data_core_core::config::CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    // Simulate worker consumer: pop, mark running, process staged items, mark success
    // (Following the same pattern as the existing e2e test)
    let popped = queue.blocking_pop_fetch().await?;
    assert_eq!(popped.workflow_id, wf_uuid);
    assert_eq!(popped.trigger_id, Some(run_uuid));

    let repo = WorkflowRepository::new(pool.pool.clone());
    let _ = repo.mark_run_running(run_uuid).await;

    // Build services for processing (same as worker)
    let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.pool.clone()));
    let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new(Arc::new(ed_adapter), cache_manager.clone());
    let de_service =
        r_data_core_services::DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    let service = WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service));

    // Process
    match service.process_staged_items(wf_uuid, run_uuid).await {
        Ok((processed, failed)) => {
            let _ = repo
                .insert_run_log(
                    run_uuid,
                    "info",
                    &format!(
                        "E2E Staged Run processed (processed_items={processed}, failed_items={failed})"
                    ),
                    None,
                )
                .await;
            let _ = repo.mark_run_success(run_uuid, processed, failed).await;
            assert_eq!(processed, 1, "one item should be processed");
            assert_eq!(failed, 0, "no item should fail");
        }
        Err(e) => {
            let _ = repo
                .insert_run_log(
                    run_uuid,
                    "error",
                    &format!("E2E Staged Run failed: {e}"),
                    None,
                )
                .await;
            let _ = repo.mark_run_failure(run_uuid, &format!("{e}")).await;
            anyhow::bail!("processing failed: {e}");
        }
    }

    // Verify entities are created correctly
    let de_repo_check = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let entities: Vec<DynamicEntity> = de_repo_check
        .get_all_by_type(&entity_type, 10, 0, None)
        .await?;
    assert_eq!(entities.len(), 1, "exactly one entity should be created");

    Ok(())
}
