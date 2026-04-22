#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::{Method::GET, MockServer};
use r_data_core_core::config::CacheConfig;
use r_data_core_persistence::{DynamicEntityRepository, OutboxRepository, WorkflowRepository};
use r_data_core_services::workflow::outbox::DispatchWorkflowOutboxBatchUseCase;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{
    create_test_admin_user, create_test_entity_definition, spawn_test_consumer_loop,
    try_setup_test_db, unique_entity_type, ConsumerLoopConfig,
};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core_workflow::data::WorkflowKind;
use std::fs;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

fn load_workflow_example(filename: &str, entity_type: &str) -> anyhow::Result<serde_json::Value> {
    let path = format!(".example_files/json_examples/dsl/{filename}");
    let content =
        fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;
    let content = content.replace("${ENTITY_TYPE}", entity_type);
    serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse {path}: {e}"))
}

#[tokio::test]
async fn end_to_end_workflow_outbox_dispatches_into_redis_and_processes_run() -> anyhow::Result<()>
{
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        eprintln!("Skipping e2e outbox test: REDIS_URL not set");
        return Ok(());
    };
    let Some(pool) = try_setup_test_db().await else {
        eprintln!("Skipping e2e outbox test: test database not available");
        return Ok(());
    };

    let fetch_key = format!("test:e2e:outbox:fetch:{}", Uuid::now_v7().simple());
    let process_key = format!("test:e2e:outbox:process:{}", Uuid::now_v7().simple());
    let email_key = format!("test:e2e:outbox:email:{}", Uuid::now_v7().simple());

    let queue_for_consumer =
        ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key, &email_key)
            .await
            .expect("Failed to create consumer queue");
    let queue_for_outbox =
        ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key, &email_key)
            .await
            .expect("Failed to create outbox queue");

    let creator_uuid = create_test_admin_user(&pool).await?;
    let entity_type = unique_entity_type("outbox_e2e_entity");
    let _entity_def_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    let mock_server = MockServer::start_async().await;
    let _mock = mock_server
        .mock_async(|when, then| {
            when.method(GET).path("/staging-data.csv");
            then.status(200)
                .header("content-type", "text/csv")
                .body("name,email\nAlice Outbox,alice.outbox@example.com\n");
        })
        .await;

    let mut cfg = load_workflow_example("workflow_csv_to_entity_simple.json", &entity_type)?;
    cfg["steps"][0]["from"]["source"]["config"]["uri"] =
        serde_json::json!(mock_server.url("/staging-data.csv"));

    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let workflow_service =
        WorkflowService::new(Arc::new(WorkflowRepositoryAdapter::new(workflow_repo)));
    let workflow_repo_check = WorkflowRepository::new(pool.pool.clone());
    let create_req = r_data_core_workflow::data::requests::CreateWorkflowRequest {
        name: format!("outbox-e2e-{}", Uuid::now_v7().simple()),
        description: Some("outbox e2e workflow".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };
    let workflow_uuid = workflow_service.create(&create_req, creator_uuid).await?;

    let (run_uuid, outbox_uuid) = workflow_service
        .enqueue_run_with_fetch_outbox(workflow_uuid)
        .await?;

    let outbox_repo = OutboxRepository::new(pool.pool.clone());
    let outbox_row_status: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(outbox_row_status, "pending");

    let cache_manager = Arc::new(r_data_core_core::cache::CacheManager::new(CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10_000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    }));
    let mut consumer_handle = spawn_test_consumer_loop(ConsumerLoopConfig {
        pool: pool.pool.clone(),
        queue: queue_for_consumer,
        cache_manager,
        fetch_key: fetch_key.clone(),
    });

    DispatchWorkflowOutboxBatchUseCase::new(
        &queue_for_outbox,
        &outbox_repo,
        "outbox-e2e-worker",
        10,
        300,
        None,
    )
    .run_once()
        .await?;

    let mut attempts = 0usize;
    loop {
        let run_status = workflow_repo_check.get_run_status(run_uuid).await?;
        if run_status.as_deref() == Some("success") {
            break;
        }
        attempts = attempts.saturating_add(1);
        if attempts > 60 {
            consumer_handle.stop();
            let _ = consumer_handle.join_handle.await;
            anyhow::bail!("workflow run did not complete in time");
        }
        sleep(Duration::from_millis(250)).await;
    }

    let entity_count = DynamicEntityRepository::new(pool.pool.clone())
        .count_entities(&entity_type)
        .await?;
    assert_eq!(entity_count, 1);

    let outbox_status: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(outbox_status, "delivered");

    consumer_handle.stop();
    let _ = consumer_handle.join_handle.await;

    Ok(())
}
