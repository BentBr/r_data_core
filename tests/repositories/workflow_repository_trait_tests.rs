#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Trait-dispatch coverage for `WorkflowRepository`.
//!
//! Exercises every `impl WorkflowRepositoryTrait` wrapper via
//! `&dyn WorkflowRepositoryTrait` so the thin delegations in
//! `workflow_repository/mod.rs` are covered, not bypassed.

use r_data_core_core::error::Result;
use r_data_core_persistence::{WorkflowRepository, WorkflowRepositoryTrait};
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use serial_test::serial;
use uuid::Uuid;

async fn seed_wf(tr: &dyn WorkflowRepositoryTrait, creator: Uuid, name: &str) -> Result<Uuid> {
    tr.create(
        &CreateWorkflowRequest {
            name: name.to_string(),
            description: None,
            kind: "consumer".to_string(),
            enabled: true,
            schedule_cron: None,
            config: serde_json::json!({}),
            versioning_disabled: false,
        },
        creator,
    )
    .await
}

async fn seed_run(tr: &dyn WorkflowRepositoryTrait, wf: Uuid) -> Result<Uuid> {
    tr.insert_run_queued(wf, Uuid::now_v7()).await
}

#[tokio::test]
#[serial]
async fn test_trait_create_and_get_by_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let uuid = seed_wf(tr, user, &random_string("tr-cg")).await?;
    let wf = tr.get_by_uuid(uuid).await?;
    assert!(wf.is_some());
    assert_eq!(wf.unwrap().uuid, uuid);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_update_and_delete() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let uuid = seed_wf(tr, user, &random_string("tr-ud")).await?;
    tr.update(
        uuid,
        &UpdateWorkflowRequest {
            name: "Trait Updated".to_string(),
            description: Some("via trait".to_string()),
            kind: "consumer".to_string(),
            enabled: false,
            schedule_cron: None,
            config: serde_json::json!({}),
            versioning_disabled: false,
        },
        user,
    )
    .await?;
    let wf = tr
        .get_by_uuid(uuid)
        .await?
        .expect("must exist after update");
    assert_eq!(wf.name, "Trait Updated");
    assert!(!wf.enabled);
    tr.delete(uuid).await?;
    assert!(tr.get_by_uuid(uuid).await?.is_none());
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_list_and_count() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    seed_wf(tr, user, &random_string("tr-lc-1")).await?;
    seed_wf(tr, user, &random_string("tr-lc-2")).await?;
    seed_wf(tr, user, &random_string("tr-lc-3")).await?;
    assert_eq!(tr.count_all().await?, 3);
    assert_eq!(tr.list_all().await?.len(), 3);
    let page = tr.list_paginated(2, 0, None, None).await?;
    assert_eq!(page.len(), 2);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_list_scheduled_consumers() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let cron_uuid = tr
        .create(
            &CreateWorkflowRequest {
                name: random_string("tr-sc"),
                description: None,
                kind: "consumer".to_string(),
                enabled: true,
                schedule_cron: Some("0 * * * *".to_string()),
                config: serde_json::json!({}),
                versioning_disabled: false,
            },
            user,
        )
        .await?;
    let scheduled = tr.list_scheduled_consumers().await?;
    assert!(scheduled.iter().any(|(u, _)| *u == cron_uuid));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_run_lifecycle_success() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-rl-s")).await?;
    let run = seed_run(tr, wf).await?;
    assert!(tr.run_exists(run).await?);
    tr.mark_run_running(run).await?;
    assert_eq!(tr.get_run_status(run).await?.as_deref(), Some("running"));
    tr.mark_run_success(run, 5, 0).await?;
    assert_eq!(tr.get_run_status(run).await?.as_deref(), Some("success"));
    assert_eq!(tr.get_workflow_uuid_for_run(run).await?, Some(wf));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_run_lifecycle_failure() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-rl-f")).await?;
    let run = seed_run(tr, wf).await?;
    tr.mark_run_failure(run, "boom").await?;
    assert_eq!(tr.get_run_status(run).await?.as_deref(), Some("failed"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_insert_run_queued_with_fetch_outbox() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-ob")).await?;
    let (run, outbox) = tr
        .insert_run_queued_with_fetch_outbox(wf, Uuid::now_v7())
        .await?;
    assert!(!run.is_nil());
    assert!(!outbox.is_nil());
    assert_ne!(run, outbox);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_list_runs_paginated() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-lrp")).await?;
    seed_run(tr, wf).await?;
    seed_run(tr, wf).await?;
    seed_run(tr, wf).await?;
    let (rows, total) = tr.list_runs_paginated(wf, 2, 0).await?;
    assert_eq!(total, 3);
    assert_eq!(rows.len(), 2);
    let (all_rows, all_total) = tr.list_all_runs_paginated(10, 0).await?;
    assert_eq!(all_total, 3);
    assert_eq!(all_rows.len(), 3);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_run_logs() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-log")).await?;
    let run = seed_run(tr, wf).await?;
    tr.insert_run_log(run, "info", "hello", None).await?;
    tr.insert_run_log(run, "error", "world", Some(serde_json::json!({"k": 1})))
        .await?;
    let (logs, total) = tr.list_run_logs_paginated(run, 10, 0).await?;
    assert_eq!(total, 2);
    assert_eq!(logs.len(), 2);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_raw_items_lifecycle() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let user = create_test_admin_user(&pool).await?;
    let wf = seed_wf(tr, user, &random_string("tr-ri")).await?;
    let run = seed_run(tr, wf).await?;
    let payloads = vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})];
    let inserted = tr.insert_raw_items(wf, run, payloads).await?;
    assert_eq!(inserted, 2);
    assert_eq!(tr.count_raw_items_for_run(run).await?, 2);
    let staged = tr.fetch_staged_raw_items(run, 10).await?;
    assert_eq!(staged.len(), 2);
    tr.set_raw_item_status(staged[0].0, "failed", Some("test error"))
        .await?;
    tr.mark_raw_items_processed(run).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_trait_unknown_uuid_probes() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;
    let repo = WorkflowRepository::new(pool.pool.clone());
    let tr: &dyn WorkflowRepositoryTrait = &repo;
    let unknown = Uuid::now_v7();
    assert!(!tr.run_exists(unknown).await?);
    assert!(tr.get_run_status(unknown).await?.is_none());
    assert!(tr.get_workflow_uuid_for_run(unknown).await?.is_none());
    Ok(())
}
