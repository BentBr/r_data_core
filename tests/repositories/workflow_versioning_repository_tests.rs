#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowVersioningRepository;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

/// Insert a minimal workflow row so FK constraints are satisfied.
/// Returns the workflow UUID.
async fn seed_workflow(pool: &PgPool, creator: Uuid) -> Uuid {
    let uuid = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO workflows (uuid, name, kind, created_by)
         VALUES ($1, $2, 'consumer'::workflow_kind, $3)",
    )
    .bind(uuid)
    .bind(random_string("wf"))
    .bind(creator)
    .execute(pool)
    .await
    .expect("failed to seed workflow");
    uuid
}

/// Bump the `version` column on a workflow so the next snapshot records a
/// new version number.
async fn bump_workflow_version(pool: &PgPool, workflow_uuid: Uuid, updated_by: Uuid) {
    sqlx::query("UPDATE workflows SET version = version + 1, updated_by = $2 WHERE uuid = $1")
        .bind(workflow_uuid)
        .bind(updated_by)
        .execute(pool)
        .await
        .expect("failed to bump workflow version");
}

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_creates_version() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    // Workflow starts at version 1 — take a snapshot
    repo.snapshot_pre_update(wf_uuid).await?;

    let versions = repo.list_workflow_versions(wf_uuid).await?;
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version_number, 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_idempotent_same_version() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    // ON CONFLICT DO NOTHING — calling twice for the same version should not fail
    repo.snapshot_pre_update(wf_uuid).await?;
    repo.snapshot_pre_update(wf_uuid).await?;

    let versions = repo.list_workflow_versions(wf_uuid).await?;
    assert_eq!(versions.len(), 1, "duplicate snapshot should be ignored");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_unknown_workflow_is_noop() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let unknown = Uuid::now_v7();

    // Should not error — just a no-op
    repo.snapshot_pre_update(unknown).await?;

    let versions = repo.list_workflow_versions(unknown).await?;
    assert!(versions.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_returns_versions_descending() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    repo.snapshot_pre_update(wf_uuid).await?;
    bump_workflow_version(&pool.pool, wf_uuid, user).await;
    repo.snapshot_pre_update(wf_uuid).await?;
    bump_workflow_version(&pool.pool, wf_uuid, user).await;
    repo.snapshot_pre_update(wf_uuid).await?;

    let versions = repo.list_workflow_versions(wf_uuid).await?;
    assert_eq!(versions.len(), 3);
    // Should be returned in descending order
    assert!(versions[0].version_number > versions[1].version_number);
    assert!(versions[1].version_number > versions[2].version_number);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_empty_for_unknown_workflow() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());

    let versions = repo.list_workflow_versions(Uuid::now_v7()).await?;
    assert!(versions.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_workflow_version_found_and_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    repo.snapshot_pre_update(wf_uuid).await?;

    let found = repo.get_workflow_version(wf_uuid, 1).await?;
    assert!(found.is_some());
    let payload = found.unwrap();
    assert_eq!(payload.version_number, 1);
    // data is the full workflow row JSON
    assert!(payload.data.is_object());

    // Non-existent version number
    let missing = repo.get_workflow_version(wf_uuid, 999).await?;
    assert!(missing.is_none());

    // Unknown workflow UUID
    let unknown = repo.get_workflow_version(Uuid::now_v7(), 1).await?;
    assert!(unknown.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_current_workflow_metadata_exists() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    let meta = repo.get_current_workflow_metadata(wf_uuid).await?;
    assert!(meta.is_some());
    let (version, _updated_at, _updated_by, _updated_by_name) = meta.unwrap();
    assert_eq!(version, 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_current_workflow_metadata_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());

    let meta = repo.get_current_workflow_metadata(Uuid::now_v7()).await?;
    assert!(meta.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_older_than_days_removes_old_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    // Manually insert an old version (created_at in the past)
    sqlx::query(
        "INSERT INTO workflow_versions (workflow_uuid, version_number, data, created_by, created_at)
         VALUES ($1, 1, '{}'::jsonb, $2, NOW() - INTERVAL '10 days')",
    )
    .bind(wf_uuid)
    .bind(user)
    .execute(&pool.pool)
    .await
    .expect("insert old version");

    let deleted = repo.prune_older_than_days(5).await?;
    assert_eq!(deleted, 1);

    let versions = repo.list_workflow_versions(wf_uuid).await?;
    assert!(versions.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_older_than_days_keeps_recent_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    repo.snapshot_pre_update(wf_uuid).await?;

    // Prune versions older than 30 days — the just-created version is recent
    let deleted = repo.prune_older_than_days(30).await?;
    assert_eq!(deleted, 0);

    let versions = repo.list_workflow_versions(wf_uuid).await?;
    assert_eq!(versions.len(), 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_keep_latest_per_workflow() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    // Create 3 version snapshots
    repo.snapshot_pre_update(wf_uuid).await?;
    bump_workflow_version(&pool.pool, wf_uuid, user).await;
    repo.snapshot_pre_update(wf_uuid).await?;
    bump_workflow_version(&pool.pool, wf_uuid, user).await;
    repo.snapshot_pre_update(wf_uuid).await?;

    // Keep only 2 — should delete 1
    let deleted = repo.prune_keep_latest_per_workflow(2).await?;
    assert_eq!(deleted, 1);

    let remaining = repo.list_workflow_versions(wf_uuid).await?;
    assert_eq!(remaining.len(), 2);
    // The two highest version numbers should survive
    assert_eq!(remaining[0].version_number, 3);
    assert_eq!(remaining[1].version_number, 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_keep_latest_noop_when_few_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let wf_uuid = seed_workflow(&pool.pool, user).await;

    repo.snapshot_pre_update(wf_uuid).await?;

    // keep=5 with only 1 version → nothing deleted
    let deleted = repo.prune_keep_latest_per_workflow(5).await?;
    assert_eq!(deleted, 0);

    Ok(())
}
