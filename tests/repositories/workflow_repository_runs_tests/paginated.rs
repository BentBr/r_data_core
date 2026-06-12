#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `list_runs_paginated`, `list_all_runs_paginated`,
//! `insert_run_log`, and `list_run_logs_paginated`.

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use uuid::Uuid;

use super::{seed_run, seed_workflow};

// ── list_runs_paginated ───────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_runs_paginated_returns_rows_and_total() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("pag-wf")).await?;

    seed_run(&repo, wf).await?;
    seed_run(&repo, wf).await?;
    seed_run(&repo, wf).await?;

    let (rows, total) = repo.list_runs_paginated(wf, 2, 0).await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(total, 3);

    let (rows2, _) = repo.list_runs_paginated(wf, 2, 2).await?;
    assert_eq!(rows2.len(), 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_runs_paginated_empty_for_unknown_workflow() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let (rows, total) = repo.list_runs_paginated(Uuid::now_v7(), 10, 0).await?;
    assert!(rows.is_empty());
    assert_eq!(total, 0);
    Ok(())
}

// ── insert_run_log + list_run_logs_paginated ──────────────────────────────────

#[tokio::test]
#[serial]
async fn test_insert_run_log_and_list_paginated() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("log-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    repo.insert_run_log(run, "info", "first message", None)
        .await?;
    repo.insert_run_log(
        run,
        "error",
        "second message",
        Some(serde_json::json!({"key": "value"})),
    )
    .await?;

    let (logs, total) = repo.list_run_logs_paginated(run, 10, 0).await?;
    assert_eq!(total, 2);
    assert_eq!(logs.len(), 2);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_run_logs_paginated_empty() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_workflow(&repo, user, &random_string("nolog-wf")).await?;
    let run = seed_run(&repo, wf).await?;

    let (logs, total) = repo.list_run_logs_paginated(run, 10, 0).await?;
    assert!(logs.is_empty());
    assert_eq!(total, 0);
    Ok(())
}

// ── list_all_runs_paginated ───────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_all_runs_paginated_spans_workflows() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf1 = seed_workflow(&repo, user, &random_string("all-wf1")).await?;
    let wf2 = seed_workflow(&repo, user, &random_string("all-wf2")).await?;

    seed_run(&repo, wf1).await?;
    seed_run(&repo, wf1).await?;
    seed_run(&repo, wf2).await?;

    let (rows, total) = repo.list_all_runs_paginated(10, 0).await?;
    assert_eq!(total, 3);
    assert_eq!(rows.len(), 3);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_all_runs_paginated_empty_db() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let (rows, total) = repo.list_all_runs_paginated(10, 0).await?;
    assert!(rows.is_empty());
    assert_eq!(total, 0);
    Ok(())
}
