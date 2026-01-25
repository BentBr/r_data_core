#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::config::CacheConfig;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::WorkflowRepositoryAdapter;
use r_data_core_services::WorkflowService;
use r_data_core_test_support::{
    create_test_admin_user, create_test_queue, setup_test_db, spawn_test_consumer_loop,
    ConsumerLoopConfig,
};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::data::WorkflowKind;
use sqlx;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn consumer_loop_processes_job_with_trigger_id() {
    let Some((queue, fetch_key, _process_key)) = create_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    let pool = setup_test_db().await;
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create admin user");

    // Create a minimal workflow
    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    let wf_uuid = wf_service
        .create(
            &r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
                name: format!("test-wf-{}", Uuid::now_v7().simple()),
                description: Some("test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await
        .expect("create workflow");

    // Create a run
    let run_uuid = wf_service.enqueue_run(wf_uuid).await.expect("enqueue run");

    // Enqueue job
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: Some(run_uuid),
        })
        .await
        .expect("enqueue job");

    // Setup cache manager
    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    // Spawn consumer loop
    let mut consumer_handle = spawn_test_consumer_loop(ConsumerLoopConfig {
        pool: pool.pool.clone(),
        queue,
        cache_manager,
        fetch_key: fetch_key.clone(),
    });

    // Wait a bit for processing
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Check run status
    let repo = WorkflowRepository::new(pool.pool.clone());
    let status = repo.get_run_status(run_uuid).await.expect("get run status");

    // Run should be processed (success or failed, but not queued)
    assert!(
        status.as_deref() != Some("queued"),
        "Run should not be queued after processing"
    );

    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;
}

#[tokio::test]
async fn consumer_loop_handles_job_without_trigger_id() {
    let Some((queue, fetch_key, _process_key)) = create_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    let pool = setup_test_db().await;
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create admin user");

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    let wf_uuid = wf_service
        .create(
            &r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
                name: format!("test-wf-{}", Uuid::now_v7().simple()),
                description: Some("test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await
        .expect("create workflow");

    // Enqueue job without trigger_id
    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: None,
        })
        .await
        .expect("enqueue job");

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    let mut consumer_handle = spawn_test_consumer_loop(ConsumerLoopConfig {
        pool: pool.pool.clone(),
        queue,
        cache_manager,
        fetch_key: fetch_key.clone(),
    });

    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify a run was created by querying the database
    let run_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(wf_uuid)
            .fetch_one(&pool.pool)
            .await
            .expect("query run count");

    assert!(run_count > 0, "A run should have been created");

    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;
}

#[tokio::test]
async fn consumer_loop_processes_multiple_jobs_sequentially() {
    let Some((queue, fetch_key, _process_key)) = create_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    let pool = setup_test_db().await;
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create admin user");

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    let wf_uuid = wf_service
        .create(
            &r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
                name: format!("test-wf-{}", Uuid::now_v7().simple()),
                description: Some("test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await
        .expect("create workflow");

    // Create multiple runs and enqueue jobs
    let mut run_uuids: Vec<Uuid> = Vec::with_capacity(3);
    for _ in 0..3 {
        let run_uuid = wf_service.enqueue_run(wf_uuid).await.expect("enqueue run");
        run_uuids.push(run_uuid);
    }

    for run_uuid in &run_uuids {
        queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: wf_uuid,
                trigger_id: Some(*run_uuid),
            })
            .await
            .expect("enqueue job");
    }

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    let mut consumer_handle = spawn_test_consumer_loop(ConsumerLoopConfig {
        pool: pool.pool.clone(),
        queue,
        cache_manager,
        fetch_key: fetch_key.clone(),
    });

    // Wait for all jobs to be processed
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Verify all runs were processed
    let repo = WorkflowRepository::new(pool.pool.clone());
    for run_uuid in &run_uuids {
        let status = repo
            .get_run_status(*run_uuid)
            .await
            .expect("get run status");
        assert!(
            status.as_deref() != Some("queued"),
            "Run {run_uuid} should not be queued after processing"
        );
    }

    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;
}

#[tokio::test]
async fn consumer_loop_continues_after_error() {
    let Some((queue, fetch_key, _process_key)) = create_test_queue().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    let pool = setup_test_db().await;
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create admin user");

    let wf_repo = WorkflowRepository::new(pool.pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    let wf_uuid = wf_service
        .create(
            &r_data_core_api::admin::workflows::models::CreateWorkflowRequest {
                name: format!("test-wf-{}", Uuid::now_v7().simple()),
                description: Some("test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await
        .expect("create workflow");

    // Enqueue job with invalid workflow_id (will fail)
    let invalid_wf_uuid = Uuid::now_v7();
    let run_uuid1 = wf_service.enqueue_run(wf_uuid).await.expect("enqueue run");

    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: invalid_wf_uuid,
            trigger_id: Some(run_uuid1),
        })
        .await
        .expect("enqueue job");

    // Enqueue valid job
    let run_uuid2 = wf_service.enqueue_run(wf_uuid).await.expect("enqueue run");

    queue
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: wf_uuid,
            trigger_id: Some(run_uuid2),
        })
        .await
        .expect("enqueue job");

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(cache_config));

    let mut consumer_handle = spawn_test_consumer_loop(ConsumerLoopConfig {
        pool: pool.pool.clone(),
        queue,
        cache_manager,
        fetch_key: fetch_key.clone(),
    });

    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Verify the valid run was processed despite the error
    let repo = WorkflowRepository::new(pool.pool.clone());
    let status2 = repo
        .get_run_status(run_uuid2)
        .await
        .expect("get run status");
    assert!(
        status2.as_deref() != Some("queued"),
        "Valid run should be processed even after error"
    );

    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;
}
