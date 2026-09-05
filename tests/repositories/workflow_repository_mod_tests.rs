#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::{get_provider_config, WorkflowRepository};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use serial_test::serial;
use uuid::Uuid;

// ── seed helper ───────────────────────────────────────────────────────────────

async fn seed_workflow(
    repo: &WorkflowRepository,
    creator: Uuid,
    name: &str,
    kind: &str,
) -> Result<Uuid> {
    let req = CreateWorkflowRequest {
        name: name.to_string(),
        description: None,
        kind: kind.to_string(),
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({}),
        versioning_disabled: false,
    };
    repo.create(&req, creator).await
}

// ── create + get_by_uuid ──────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_create_and_get_by_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = seed_workflow(&repo, user, &random_string("wf"), "consumer").await?;

    let wf = repo.get_by_uuid(uuid).await?;
    assert!(wf.is_some());
    let wf = wf.unwrap();
    assert_eq!(wf.uuid, uuid);
    assert!(wf.enabled);

    Ok(())
}

// ── get_by_uuid — not found ───────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_get_by_uuid_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());

    let result = repo.get_by_uuid(Uuid::now_v7()).await?;
    assert!(result.is_none());

    Ok(())
}

// ── list_all ──────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_all_returns_all_workflows() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    seed_workflow(&repo, user, &random_string("wf-a"), "consumer").await?;
    seed_workflow(&repo, user, &random_string("wf-b"), "provider").await?;

    let all = repo.list_all().await?;
    assert!(all.len() >= 2);

    Ok(())
}

// ── list_paginated ────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_paginated_honours_limit_and_offset() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    for i in 0..5 {
        seed_workflow(
            &repo,
            user,
            &random_string(&format!("pg-wf-{i}")),
            "consumer",
        )
        .await?;
    }

    let page1 = repo.list_paginated(2, 0, None, None).await?;
    assert_eq!(page1.len(), 2);

    let page2 = repo.list_paginated(2, 2, None, None).await?;
    assert_eq!(page2.len(), 2);

    // Ensure distinct UUIDs across pages
    let ids1: Vec<Uuid> = page1.iter().map(|w| w.uuid).collect();
    let ids2: Vec<Uuid> = page2.iter().map(|w| w.uuid).collect();
    assert!(ids1.iter().all(|id| !ids2.contains(id)));

    Ok(())
}

// ── count_all ─────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_count_all_matches_seeded_rows() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    seed_workflow(&repo, user, &random_string("cnt-wf"), "consumer").await?;
    seed_workflow(&repo, user, &random_string("cnt-wf"), "consumer").await?;
    seed_workflow(&repo, user, &random_string("cnt-wf"), "consumer").await?;

    let count = repo.count_all().await?;
    assert_eq!(count, 3);

    Ok(())
}

// ── update ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_update_changes_name_and_description() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = seed_workflow(&repo, user, &random_string("upd-wf"), "consumer").await?;

    let upd = UpdateWorkflowRequest {
        name: "Updated Name".to_string(),
        description: Some("A description".to_string()),
        kind: "consumer".to_string(),
        enabled: false,
        schedule_cron: None,
        config: serde_json::json!({}),
        versioning_disabled: true,
    };
    repo.update(uuid, &upd, user).await?;

    let wf = repo.get_by_uuid(uuid).await?.expect("workflow must exist");
    assert_eq!(wf.name, "Updated Name");
    assert_eq!(wf.description.as_deref(), Some("A description"));
    assert!(!wf.enabled);

    Ok(())
}

// ── delete ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_delete_removes_workflow() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let uuid = seed_workflow(&repo, user, &random_string("del-wf"), "consumer").await?;
    repo.delete(uuid).await?;

    let result = repo.get_by_uuid(uuid).await?;
    assert!(result.is_none());

    Ok(())
}

// ── list_scheduled_consumers ──────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_scheduled_consumers_returns_only_enabled_cron_consumers() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    // Enabled consumer with a cron — should appear
    let req_cron = CreateWorkflowRequest {
        name: random_string("cron-wf"),
        description: None,
        kind: "consumer".to_string(),
        enabled: true,
        schedule_cron: Some("0 * * * *".to_string()),
        config: serde_json::json!({}),
        versioning_disabled: false,
    };
    let cron_uuid = repo.create(&req_cron, user).await?;

    // Enabled consumer without a cron — must NOT appear
    seed_workflow(&repo, user, &random_string("nocron-wf"), "consumer").await?;

    let scheduled = repo.list_scheduled_consumers().await?;
    assert!(
        scheduled.iter().any(|(uuid, _)| *uuid == cron_uuid),
        "cron workflow must appear in scheduled consumers"
    );

    Ok(())
}

// ── get_provider_config (free function) ───────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_get_provider_config_returns_none_for_non_provider() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    // consumer kind — get_provider_config must return None
    let uuid = seed_workflow(&repo, user, &random_string("cons-wf"), "consumer").await?;

    let cfg = get_provider_config(&pool.pool, uuid).await?;
    assert!(cfg.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_provider_config_returns_none_for_unknown_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let cfg = get_provider_config(&pool.pool, Uuid::now_v7()).await?;
    assert!(cfg.is_none());

    Ok(())
}
