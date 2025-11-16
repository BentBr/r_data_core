use r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use r_data_core::entity::dynamic_entity::DynamicEntityRepositoryTrait;
use r_data_core::entity::DynamicEntity;
use r_data_core::services::{
    adapters::EntityDefinitionRepositoryAdapter, DynamicEntityRepositoryAdapter,
    EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core::workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use r_data_core::workflow::data::job_queue::JobQueue;
use r_data_core::workflow::data::jobs::FetchAndStageJob;
use r_data_core::workflow::data::repository::WorkflowRepository;
use r_data_core::workflow::data::WorkflowKind;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// Import common test utilities
#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn end_to_end_workflow_processing_via_redis_queue() -> anyhow::Result<()> {
    // Skip test if REDIS_URL not present
    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Skipping e2e test: REDIS_URL not set");
            return Ok(());
        }
    };

    // Use unique keys per test to avoid cross-test interference
    let fetch_key = format!("test:e2e:queue:fetch:{}", Uuid::now_v7());
    let process_key = format!("test:e2e:queue:process:{}", Uuid::now_v7());
    let queue = ApalisRedisQueue::from_parts(&redis_url, &fetch_key, &process_key)
        .await
        .expect("Failed to create Redis queue for e2e test");

    // DB setup
    let pool: PgPool = common::utils::setup_test_db().await;

    // Create a dynamic entity definition used by the workflow's "to" step
    let entity_type = common::utils::unique_entity_type("e2e_entity");
    let _entity_def_uuid =
        common::utils::create_test_entity_definition(&pool, &entity_type).await?;

    // Create a workflow that maps CSV to the dynamic entity
    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let wf_service = WorkflowService::new(Arc::new(wf_adapter));

    // Resolve a creator (admin user) or use a generated UUID for created_by
    let creator_uuid = common::utils::create_test_admin_user(&pool).await?;

    let cfg = serde_json::json!({
        "steps": [
            {
                "from": { "type": "csv", "uri": "inline://provided-by-staging", "mapping": {} },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": entity_type,
                    "path": "/",
                    "mode": "create",
                    "mapping": {
                        "name": "name",
                        "email": "email"
                    }
                }
            }
        ]
    });

    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("e2e-wf-{}", Uuid::now_v7()),
        description: Some("e2e workflow".to_string()),
        kind: WorkflowKind::Consumer,
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

    let repo = WorkflowRepository::new(pool.clone());
    let _ = repo.mark_run_running(run_uuid).await;

    // Build services for processing (same as worker)
    let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
    let de_repo =
        r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository::new(pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service = EntityDefinitionService::new_without_cache(Arc::new(ed_adapter));
    let de_service = r_data_core::services::DynamicEntityService::new(
        Arc::new(de_adapter),
        Arc::new(ed_service),
    );
    let service = WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service));

    // Process
    match service.process_staged_items(wf_uuid, run_uuid).await {
        Ok((processed, failed)) => {
            let _ = repo
                .insert_run_log(
                    run_uuid,
                    "info",
                    &format!(
                        "E2E Run processed (processed_items={}, failed_items={})",
                        processed, failed
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
                .insert_run_log(run_uuid, "error", &format!("E2E Run failed: {}", e), None)
                .await;
            let _ = repo.mark_run_failure(run_uuid, &format!("{}", e)).await;
            anyhow::bail!("processing failed: {}", e);
        }
    }

    // Validate output: the dynamic entity was created
    let de_repo_check =
        r_data_core::entity::dynamic_entity::repository::DynamicEntityRepository::new(pool.clone());
    let entities: Vec<DynamicEntity> = de_repo_check
        .get_all_by_type(&entity_type, 10, 0, None)
        .await?;
    assert_eq!(entities.len(), 1, "exactly one entity should be created");

    Ok(())
}
