#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::{bump_definition_version, seed_entity_definition};
use r_data_core_core::error::Result;
use r_data_core_persistence::EntityDefinitionVersioningRepository;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// get_definition_version
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_definition_version_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;

    let found = repo.get_definition_version(def_uuid, 1).await?;
    assert!(found.is_some(), "version 1 must exist after snapshot");
    let payload = found.unwrap();
    assert_eq!(payload.version_number, 1);
    assert!(
        payload.data.is_object(),
        "snapshot data must be a JSON object"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_definition_version_not_found_wrong_version() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    repo.snapshot_pre_update(def_uuid).await?;

    let missing = repo.get_definition_version(def_uuid, 999).await?;
    assert!(missing.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_definition_version_not_found_unknown_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());

    let result = repo.get_definition_version(Uuid::now_v7(), 1).await?;
    assert!(result.is_none());

    Ok(())
}

// ---------------------------------------------------------------------------
// get_current_definition_metadata
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_current_definition_metadata_exists() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    let meta = repo.get_current_definition_metadata(def_uuid).await?;
    assert!(meta.is_some());
    let (version, _updated_at, _updated_by, _name) = meta.unwrap();
    assert_eq!(version, 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_current_definition_metadata_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());

    let meta = repo.get_current_definition_metadata(Uuid::now_v7()).await?;
    assert!(meta.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_current_definition_metadata_reflects_bump() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = EntityDefinitionVersioningRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;
    let def_uuid = seed_entity_definition(&pool.pool, user).await;

    bump_definition_version(&pool.pool, def_uuid, user).await;

    let meta = repo.get_current_definition_metadata(def_uuid).await?;
    let (version, _, updated_by, _) = meta.unwrap();
    assert_eq!(version, 2, "version must reflect the bump");
    assert_eq!(updated_by, Some(user));

    Ok(())
}
