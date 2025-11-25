use r_data_core_api::admin::workflows::models::CreateWorkflowRequest;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_workflow::data::WorkflowKind;
use uuid::Uuid;

// Import the common module from tests
#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn get_workflow_uuid_for_run_round_trip() -> anyhow::Result<()> {
    // Setup test database
    let pool = common::utils::setup_test_db().await;

    let repo = WorkflowRepository::new(pool.clone());

    // Resolve a creator (admin user) from DB
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("fetch admin user uuid");

    // Create a workflow
    let req = CreateWorkflowRequest {
        name: format!("worker-test-{}", Uuid::now_v7()),
        description: Some("worker test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: Some("*/5 * * * *".to_string()),
        config: serde_json::json!({
            "steps": [
                {
                    "from": { "type": "csv", "uri": "http://example.com/data.csv", "mapping": {} },
                    "transform": { "type": "none" },
                    "to": { "type": "json", "output": "api", "mapping": {} }
                }
            ]
        }),
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&req, creator_uuid).await?;

    // Enqueue a run
    let trigger_id = Uuid::now_v7();
    let run_uuid = repo.insert_run_queued(wf_uuid, trigger_id).await?;

    // Resolve via repository
    let resolved = repo.get_workflow_uuid_for_run(run_uuid).await?;
    assert_eq!(resolved, Some(wf_uuid));
    Ok(())
}
