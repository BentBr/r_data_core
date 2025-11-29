use r_data_core_api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::WorkflowRepositoryAdapter;
use r_data_core_workflow::data::WorkflowKind;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_test_support::{create_test_admin_user, setup_test_db};

#[tokio::test]
async fn create_sets_created_by_and_fk_enforced() -> anyhow::Result<()> {
    let pool = setup_test_db().await;

    // Ensure we have at least one admin user to act as creator
    let creator_uuid = create_test_admin_user(&pool).await?;

    let repo = WorkflowRepository::new(pool.clone());
    let adapter = WorkflowRepositoryAdapter::new(repo);
    let _svc = r_data_core_services::WorkflowService::new(Arc::new(adapter));

    // Minimal valid DSL config
    let cfg = serde_json::json!({
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
    });

    let req = CreateWorkflowRequest {
        name: format!("wf-create-{}", Uuid::now_v7()),
        description: Some("audit test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg,
        versioning_disabled: false,
    };

    // Create via repository (adapter only used to match service wiring)
    let repo_core = WorkflowRepository::new(pool.clone());
    let wf_uuid = repo_core.create(&req, creator_uuid).await?;

    // Verify created_by stored
    let row = sqlx::query("SELECT created_by FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await?;
    let stored_created_by: Uuid = row.try_get("created_by")?;
    assert_eq!(stored_created_by, creator_uuid);

    // Try to delete the admin user -> expect FK restriction error
    let delete_res = sqlx::query("DELETE FROM admin_users WHERE uuid = $1")
        .bind(creator_uuid)
        .execute(&pool)
        .await;
    assert!(
        delete_res.is_err(),
        "Expected FK restriction to prevent deleting referenced admin user"
    );

    Ok(())
}

#[tokio::test]
async fn update_sets_updated_by_and_fk_enforced() -> anyhow::Result<()> {
    let pool = setup_test_db().await;

    let creator_uuid = create_test_admin_user(&pool).await?;
    let updater_uuid = create_test_admin_user(&pool).await?;

    let repo = WorkflowRepository::new(pool.clone());

    // Minimal valid config
    let cfg = serde_json::json!({
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
    });

    // Create base workflow
    let create_req = CreateWorkflowRequest {
        name: format!("wf-update-{}", Uuid::now_v7()),
        description: Some("audit test".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: true,
        schedule_cron: None,
        config: cfg.clone(),
        versioning_disabled: false,
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Update and set updated_by
    let update_req = UpdateWorkflowRequest {
        name: format!("wf-update-{}-edited", Uuid::now_v7()),
        description: Some("edited".to_string()),
        kind: WorkflowKind::Consumer.to_string(),
        enabled: false,
        schedule_cron: Some("*/5 * * * *".to_string()),
        config: cfg,
        versioning_disabled: false,
    };
    repo.update(wf_uuid, &update_req, updater_uuid).await?;

    // Verify updated_by stored
    let row = sqlx::query("SELECT updated_by FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await?;
    let stored_updated_by: Option<Uuid> = row.try_get("updated_by")?;
    assert_eq!(stored_updated_by, Some(updater_uuid));

    // Deleting updater should be restricted
    let delete_res = sqlx::query("DELETE FROM admin_users WHERE uuid = $1")
        .bind(updater_uuid)
        .execute(&pool)
        .await;
    assert!(
        delete_res.is_err(),
        "Expected FK restriction to prevent deleting referenced admin user (updated_by)"
    );

    Ok(())
}
