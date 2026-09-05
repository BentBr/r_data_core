#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Basic CRUD tests: create, get, count, update, delete.

use super::{seed, seed_with_cron};
use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use serial_test::serial;
use uuid::Uuid;

// ── create with all optional fields ──────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_create_with_description_and_cron() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = repo
        .create(
            &CreateWorkflowRequest {
                name: random_string("full-wf"),
                description: Some("A full workflow".to_string()),
                kind: "consumer".to_string(),
                enabled: true,
                schedule_cron: Some("*/5 * * * *".to_string()),
                config: serde_json::json!({"key": "value"}),
                versioning_disabled: true,
            },
            user,
        )
        .await?;

    let wf = repo.get_by_uuid(uuid).await?.expect("must exist");
    assert_eq!(wf.description.as_deref(), Some("A full workflow"));
    assert_eq!(wf.schedule_cron.as_deref(), Some("*/5 * * * *"));
    assert!(wf.versioning_disabled);
    assert_eq!(wf.config["key"], "value");

    Ok(())
}

// ── create provider kind ──────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_create_provider_kind() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = seed(&repo, user, &random_string("prov-wf"), "provider").await?;
    let wf = repo.get_by_uuid(uuid).await?.expect("must exist");
    assert_eq!(format!("{:?}", wf.kind).to_lowercase(), "provider");

    Ok(())
}

// ── count_all on empty DB ─────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_count_all_returns_zero_on_empty_db() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let count = repo.count_all().await?;
    assert_eq!(count, 0);

    Ok(())
}

// ── delete non-existent UUID does not error ───────────────────────────────────

#[tokio::test]
#[serial]
async fn test_delete_non_existent_uuid_ok() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    // Should succeed silently (0 rows affected)
    repo.delete(Uuid::now_v7()).await?;

    Ok(())
}

// ── update clears description ─────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_update_clears_description() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = repo
        .create(
            &CreateWorkflowRequest {
                name: random_string("desc-wf"),
                description: Some("Initial desc".to_string()),
                kind: "consumer".to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({}),
                versioning_disabled: false,
            },
            user,
        )
        .await?;

    repo.update(
        uuid,
        &UpdateWorkflowRequest {
            name: random_string("desc-wf-updated"),
            description: None,
            kind: "consumer".to_string(),
            enabled: true,
            schedule_cron: None,
            config: serde_json::json!({}),
            versioning_disabled: false,
        },
        user,
    )
    .await?;

    let wf = repo.get_by_uuid(uuid).await?.expect("must exist");
    assert!(wf.description.is_none(), "description must be cleared");

    Ok(())
}

// ── list_scheduled_consumers: disabled workflow not included ──────────────────

#[tokio::test]
#[serial]
async fn test_list_scheduled_consumers_excludes_disabled_workflow() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let disabled_uuid = seed_with_cron(
        &repo,
        user,
        &random_string("disabled-wf"),
        serde_json::json!({}),
        false,
    )
    .await?;

    let scheduled = repo.list_scheduled_consumers().await?;
    assert!(
        !scheduled.iter().any(|(u, _)| *u == disabled_uuid),
        "disabled workflow must not appear in scheduled consumers"
    );

    Ok(())
}
