#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::{bump_definition_version, seed_entity_definition};
use r_data_core_core::error::Result;
use r_data_core_persistence::EntityDefinitionVersioningRepository;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

// ---------------------------------------------------------------------------
// prune_older_than_days
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_prune_older_than_days_removes_old_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    // Manually insert a version with a past timestamp
    sqlx::query(
        "INSERT INTO entity_definition_versions
         (definition_uuid, version_number, data, created_by, created_at)
         VALUES ($1, 1, '{}'::jsonb, $2, NOW() - INTERVAL '10 days')",
    )
    .bind(def_uuid)
    .bind(user)
    .execute(&pool.pool)
    .await
    .expect("insert old version");

    let deleted = repo.prune_older_than_days(5).await?;
    assert_eq!(deleted, 1);

    let remaining = repo.list_definition_versions(def_uuid).await?;
    assert!(remaining.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_older_than_days_keeps_recent_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;

    // Prune with a large window — the just-created version must survive
    let deleted = repo.prune_older_than_days(30).await?;
    assert_eq!(deleted, 0);

    let remaining = repo.list_definition_versions(def_uuid).await?;
    assert_eq!(remaining.len(), 1);

    Ok(())
}

// ---------------------------------------------------------------------------
// prune_keep_latest_per_definition
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_prune_keep_latest_per_definition_removes_oldest() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    // Create 3 version snapshots
    repo.snapshot_pre_update(def_uuid).await?;
    bump_definition_version(&pool.pool, def_uuid, user).await;
    repo.snapshot_pre_update(def_uuid).await?;
    bump_definition_version(&pool.pool, def_uuid, user).await;
    repo.snapshot_pre_update(def_uuid).await?;

    // Keep only 2 — should delete 1
    let deleted = repo.prune_keep_latest_per_definition(2).await?;
    assert_eq!(deleted, 1);

    let remaining = repo.list_definition_versions(def_uuid).await?;
    assert_eq!(remaining.len(), 2);
    // Highest two version numbers must survive
    assert_eq!(remaining[0].version_number, 3);
    assert_eq!(remaining[1].version_number, 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prune_keep_latest_noop_when_few_versions() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;

    // keep=5 with only 1 version — nothing deleted
    let deleted = repo.prune_keep_latest_per_definition(5).await?;
    assert_eq!(deleted, 0);

    Ok(())
}
