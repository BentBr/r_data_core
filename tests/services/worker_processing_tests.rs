use r_data_core_api::admin::workflows::models::CreateWorkflowRequest;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::WorkflowRepositoryAdapter;
use r_data_core_services::WorkflowService;
use r_data_core_workflow::data::WorkflowKind;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn run_now_creates_queued_run_and_worker_marks_success() {
    // Setup DB connection (requires test database env)
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set for tests");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect db");

    let repo = WorkflowRepository::new(pool.clone());
    let adapter = WorkflowRepositoryAdapter::new(repo);
    let service = WorkflowService::new(Arc::new(adapter));

    // Resolve creator (admin user)
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("fetch admin user");

    // Create a consumer workflow (enabled, no cron) with minimal valid DSL config
    let req = CreateWorkflowRequest {
        name: format!("test-wf-{}", Uuid::now_v7()),
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
    };
    let wf_uuid = service
        .create(&req, creator_uuid)
        .await
        .expect("create workflow");

    // Simulate run enqueue
    let repo_core = WorkflowRepository::new(pool.clone());
    let trigger_id = Uuid::now_v7();
    repo_core
        .insert_run_queued(wf_uuid, trigger_id)
        .await
        .expect("insert queued");

    // Process one tick like worker loop would do
    if let Ok(run_ids) = repo_core.list_queued_runs(10).await {
        for run in run_ids {
            let _ = repo_core.mark_run_running(run).await;
            let _ = repo_core.mark_run_success(run, 0, 0).await;
        }
    }

    // Assert run is marked success
    let row = sqlx::query!(
        r#"SELECT status::text AS status FROM workflow_runs WHERE workflow_uuid = $1 ORDER BY queued_at DESC LIMIT 1"#,
        wf_uuid
    )
    .fetch_one(&pool)
    .await
    .expect("fetch run");

    assert_eq!(row.status.as_deref(), Some("success"));
}
