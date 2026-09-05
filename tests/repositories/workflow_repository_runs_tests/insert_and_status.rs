#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for insert, queue listing, status transitions, run existence,
//! and `get_workflow_uuid_for_run`.

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use uuid::Uuid;

use super::{seed_run, seed_workflow};

// ── insert_run_queued ─────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_insert_run_queued_returns_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("run-wf")).await?;

    let run_uuid = seed_run(&repo, wf).await?;
    assert!(!run_uuid.is_nil());
    Ok(())
}

// ── insert_run_queued_with_fetch_outbox ───────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_insert_run_queued_with_fetch_outbox_returns_pair() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("ob-wf")).await?;

    let (run_uuid, outbox_uuid) = repo
        .insert_run_queued_with_fetch_outbox(wf, Uuid::now_v7())
        .await?;
    assert!(!run_uuid.is_nil());
    assert!(!outbox_uuid.is_nil());
    assert_ne!(run_uuid, outbox_uuid);
    Ok(())
}

// ── list_queued_runs ──────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_queued_runs_returns_newly_inserted() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("lq-wf")).await?;

    let run1 = seed_run(&repo, wf).await?;
    let run2 = seed_run(&repo, wf).await?;

    let queued = repo.list_queued_runs(100).await?;
    assert!(queued.contains(&run1));
    assert!(queued.contains(&run2));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_queued_runs_respects_limit() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("lim-wf")).await?;

    for _ in 0..5 {
        seed_run(&repo, wf).await?;
    }

    let queued = repo.list_queued_runs(2).await?;
    assert_eq!(queued.len(), 2);
    Ok(())
}

// ── status transitions ────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_mark_run_running_changes_status() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("run-st-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    repo.mark_run_running(run).await?;

    let status = repo.get_run_status(run).await?;
    assert_eq!(status.as_deref(), Some("running"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mark_run_success_sets_status() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("succ-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    repo.mark_run_running(run).await?;
    repo.mark_run_success(run, 10, 2).await?;

    let status = repo.get_run_status(run).await?;
    assert_eq!(status.as_deref(), Some("success"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mark_run_failure_sets_status() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("fail-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    repo.mark_run_failure(run, "something went wrong").await?;

    let status = repo.get_run_status(run).await?;
    assert_eq!(status.as_deref(), Some("failed"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_run_status_not_found_returns_none() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let status = repo.get_run_status(Uuid::now_v7()).await?;
    assert!(status.is_none());
    Ok(())
}

// ── run_exists ────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_run_exists_true_for_known_run() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("exists-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    assert!(repo.run_exists(run).await?);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_run_exists_false_for_unknown_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    assert!(!repo.run_exists(Uuid::now_v7()).await?);
    Ok(())
}

// ── get_workflow_uuid_for_run ─────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_get_workflow_uuid_for_run_returns_parent() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("gwf-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    let found = repo.get_workflow_uuid_for_run(run).await?;
    assert_eq!(found, Some(wf));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_workflow_uuid_for_run_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let found = repo.get_workflow_uuid_for_run(Uuid::now_v7()).await?;
    assert!(found.is_none());
    Ok(())
}
