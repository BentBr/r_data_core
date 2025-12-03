#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_api::admin::workflows::models::CreateWorkflowRequest;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use r_data_core_workflow::data::WorkflowKind;
use uuid::Uuid;

#[tokio::test]
async fn get_workflow_uuid_for_run_round_trip() -> anyhow::Result<()> {
    // Setup test database
    let pool = setup_test_db().await;

    let repo = WorkflowRepository::new(pool.clone());

    // Create a test admin user
    let creator_uuid = create_test_admin_user(&pool)
        .await
        .expect("create test admin user");

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
