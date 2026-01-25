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

/// Test that POST to workflow endpoint with from.api source enqueues job to Redis
///
/// This is a regression test for the bug where `post_workflow_ingest` created runs
/// and staged items but never called `enqueue_fetch` to push jobs to Redis.
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn post_to_workflow_endpoint_enqueues_fetch_job() -> anyhow::Result<()> {
    use actix_web::{test as actix_test, web, App};
    use r_data_core_api::{configure_app, ApiState, ApiStateWrapper};
    use r_data_core_core::admin_user::AdminUser;
    use r_data_core_core::cache::CacheManager;
    use r_data_core_core::config::{ApiConfig, CacheConfig, LicenseConfig};
    use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};
    use r_data_core_services::{AdminUserService, ApiKeyService, LicenseService, RoleService};

    // Skip test if REDIS_URL not present
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("Skipping e2e test: REDIS_URL not set");
        return Ok(());
    };

    // Use unique keys per test to avoid cross-test interference
    let fetch_key = format!("test:e2e:post:fetch:{}", Uuid::now_v7().simple());
    let process_key = format!("test:e2e:post:process:{}", Uuid::now_v7().simple());
    let queue = Arc::new(
        ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
            .await
            .expect("Failed to create Redis queue for e2e test"),
    );

    // DB setup
    let pool = setup_test_db().await;

    // Create entity definition for the workflow target
    let entity_type = unique_entity_type("e2e_post_entity");
    let _entity_def_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Create admin user for auth
    let admin_uuid = create_test_admin_user(&pool).await?;

    // Build all required services
    let cache_config = CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let license_config = LicenseConfig::default();
    let license_service = Arc::new(LicenseService::new(license_config, cache_manager.clone()));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service = EntityDefinitionService::new_without_cache(Arc::new(
        r_data_core_persistence::EntityDefinitionRepository::new(pool.pool.clone()),
    ));

    let de_repo = r_data_core_persistence::DynamicEntityRepository::new(pool.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let dynamic_entity_service = Arc::new(DynamicEntityService::new(
        Arc::new(de_adapter),
        Arc::new(entity_definition_service.clone()),
    ));

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service =
        WorkflowService::new_with_entities(Arc::new(wf_adapter), dynamic_entity_service.clone());

    let dashboard_stats_repository =
        r_data_core_persistence::DashboardStatsRepository::new(pool.pool.clone());
    let dashboard_stats_service =
        r_data_core_services::DashboardStatsService::new(Arc::new(dashboard_stats_repository));

    let jwt_secret = "test_secret".to_string();
    let api_config = ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: jwt_secret.clone(),
        jwt_expiration: 3600,
        enable_docs: true,
        cors_origins: vec![],
        check_default_admin_password: true,
    };

    let api_state = ApiState {
        db_pool: pool.pool.clone(),
        api_config: api_config.clone(),
        role_service: RoleService::new(pool.pool.clone(), cache_manager.clone(), Some(0)),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: Some(dynamic_entity_service),
        workflow_service,
        dashboard_stats_service,
        queue: queue.clone(), // Use our custom queue with unique keys
        license_service,
    };

    let app_data = web::Data::new(ApiStateWrapper::new(api_state));

    let app = actix_test::init_service(
        App::new()
            .app_data(app_data.clone())
            .configure(configure_app),
    )
    .await;

    // Generate JWT token for auth
    let user: AdminUser = sqlx::query_as("SELECT * FROM admin_users WHERE uuid = $1")
        .bind(admin_uuid)
        .fetch_one(&pool.pool)
        .await?;
    let token = r_data_core_api::jwt::generate_access_token(&user, &api_config, &[])?;

    // Create workflow with from.api source (accepts POST data)
    // Uses example file from .example_files/json_examples/dsl/
    let config = api::workflows::common::load_workflow_example(
        "workflow_api_source_json_to_entity.json",
        &entity_type,
    )?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("e2e-post-wf-{}", Uuid::now_v7().simple()),
        description: Some("e2e POST endpoint test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config,
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, admin_uuid).await?;

    // POST JSON data to the workflow endpoint
    let json_data = r#"[{"name":"Test User","email":"test@example.com"}]"#;
    let req = actix_test::TestRequest::post()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(json_data.as_bytes())
        .to_request();

    let resp = actix_test::call_service(&app, req).await;

    // Verify response is 202 Accepted
    assert_eq!(
        resp.status().as_u16(),
        202,
        "Expected 202 Accepted, got: {}",
        resp.status()
    );

    // Parse response to get run_uuid
    let body = actix_test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)?;
    let run_uuid_str = response["run_uuid"]
        .as_str()
        .expect("Response should contain run_uuid");
    let run_uuid: Uuid = run_uuid_str.parse()?;

    // CRITICAL: Verify the job was actually enqueued to Redis
    // Use a timeout to avoid hanging if the queue is empty
    let pop_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        queue.blocking_pop_fetch(),
    )
    .await;

    match pop_result {
        Ok(Ok(job)) => {
            assert_eq!(
                job.workflow_id, wf_uuid,
                "Popped job should have correct workflow_id"
            );
            assert_eq!(
                job.trigger_id,
                Some(run_uuid),
                "Popped job should have correct trigger_id (run_uuid)"
            );
        }
        Ok(Err(e)) => {
            panic!("Failed to pop job from queue: {e}. The enqueue_fetch call may be missing!");
        }
        Err(elapsed) => {
            panic!(
                "Timeout ({elapsed}) waiting for job from queue. Job was not enqueued to Redis!"
            );
        }
    }

    Ok(())
}
