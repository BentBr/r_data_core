#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::{bump_definition_version, seed_entity_definition};
use r_data_core_core::error::Result;
use r_data_core_persistence::EntityDefinitionVersioningRepository;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// snapshot_pre_update
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_creates_version() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;

    let versions = repo.list_definition_versions(def_uuid).await?;
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version_number, 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_idempotent_same_version() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    // ON CONFLICT DO NOTHING — two snapshots of the same version must not error
    repo.snapshot_pre_update(def_uuid).await?;
    repo.snapshot_pre_update(def_uuid).await?;

    let versions = repo.list_definition_versions(def_uuid).await?;
    assert_eq!(versions.len(), 1, "duplicate snapshot must be deduplicated");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_snapshot_pre_update_unknown_definition_is_noop() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let unknown = Uuid::now_v7();

    // Must not error for a non-existent definition
    repo.snapshot_pre_update(unknown).await?;

    let versions = repo.list_definition_versions(unknown).await?;
    assert!(versions.is_empty());

    Ok(())
}

// ---------------------------------------------------------------------------
// list_definition_versions
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_returns_versions_descending() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;
    bump_definition_version(&pool.pool, def_uuid, user).await;
    repo.snapshot_pre_update(def_uuid).await?;
    bump_definition_version(&pool.pool, def_uuid, user).await;
    repo.snapshot_pre_update(def_uuid).await?;

    let versions = repo.list_definition_versions(def_uuid).await?;
    assert_eq!(versions.len(), 3);
    // ORDER BY version_number DESC
    assert!(versions[0].version_number > versions[1].version_number);
    assert!(versions[1].version_number > versions[2].version_number);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_empty_for_unknown_definition() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());

    let versions = repo.list_definition_versions(Uuid::now_v7()).await?;
    assert!(versions.is_empty());

    Ok(())
}
