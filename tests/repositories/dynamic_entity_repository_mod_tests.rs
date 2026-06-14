#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
// Seed helpers propagate errors; doc comments would only add noise.
#![allow(clippy::missing_errors_doc)]

//! Integration tests for `dynamic_entity_repository/mod.rs` public methods
//! that are not covered by the existing CRUD / query / additional test files.
//!
//! Covered here:
//! - `new` constructor (field defaults)
//! - `with_cache` constructor (cache manager attached)
//! - `count_children`
//! - `find_one_by_filters`
//! - `get_raw_field_value` (found + not-found + invalid-field)
//! - `get_by_uuid_any_type` (via `DynamicEntityRepositoryTrait`)

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::setup_test_db;

use super::dynamic_entity_query_repository_tests::{make_entity, setup_entity_type};

// ── constructor: new ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_new_has_no_cache_manager() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    assert!(repo.cache_manager.is_none());
    Ok(())
}

// ── constructor: with_cache ───────────────────────────────────────────────────

#[tokio::test]
async fn test_with_cache_attaches_manager() -> Result<()> {
    use r_data_core_core::cache::CacheManager;
    use r_data_core_core::config::CacheConfig;

    let pool = setup_test_db().await;

    // Build a minimal CacheManager (in-memory only) — we only check that the
    // Arc is stored, not that Redis is reachable.
    let cm = Arc::new(CacheManager::new(CacheConfig::default()));
    let repo = DynamicEntityRepository::with_cache(pool.pool.clone(), Arc::clone(&cm));
    assert!(repo.cache_manager.is_some());
    Ok(())
}

// ── count_children ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_count_children_zero_before_any_child() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "cc_base").await?;

    let parent = make_entity(&def, "parent", 1);
    let parent_uuid = repo.create(&parent).await?;

    let count = repo.count_children(&parent_uuid).await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn test_count_children_increments_after_child_added() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "cc_incr").await?;

    let parent = make_entity(&def, "parent", 10);
    let parent_uuid = repo.create(&parent).await?;

    let mut child = make_entity(&def, "child", 5);
    child.set("parent_uuid", parent_uuid.to_string())?;
    repo.create(&child).await?;

    let count = repo.count_children(&parent_uuid).await?;
    assert_eq!(count, 1);
    Ok(())
}

#[tokio::test]
async fn test_count_children_unknown_uuid_returns_zero() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let count = repo.count_children(&Uuid::now_v7()).await?;
    assert_eq!(count, 0);
    Ok(())
}

// ── find_one_by_filters ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_find_one_by_filters_returns_matching_entity() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "fof_match").await?;

    let entity = make_entity(&def, "UniqueFilterName", 42);
    let uuid = repo.create(&entity).await?;

    let filters = HashMap::from([("name".to_string(), json!("UniqueFilterName"))]);
    let found = repo.find_one_by_filters(&def.entity_type, &filters).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().get::<Uuid>("uuid")?, uuid);
    Ok(())
}

#[tokio::test]
async fn test_find_one_by_filters_returns_none_when_no_match() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "fof_nomatch").await?;

    let filters = HashMap::from([("name".to_string(), json!("DoesNotExist"))]);
    let found = repo.find_one_by_filters(&def.entity_type, &filters).await?;
    assert!(found.is_none());
    Ok(())
}

// ── get_raw_field_value ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_raw_field_value_returns_stored_string() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "grfv_found").await?;

    let entity = make_entity(&def, "RawValue", 7);
    let uuid = repo.create(&entity).await?;

    let raw = repo
        .get_raw_field_value(&def.entity_type, &uuid, "name")
        .await?;
    assert_eq!(raw.as_deref(), Some("RawValue"));
    Ok(())
}

#[tokio::test]
async fn test_get_raw_field_value_returns_none_for_missing_uuid() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "grfv_miss").await?;

    let raw = repo
        .get_raw_field_value(&def.entity_type, &Uuid::now_v7(), "name")
        .await?;
    assert!(raw.is_none());
    Ok(())
}

#[tokio::test]
async fn test_get_raw_field_value_errors_on_invalid_field() -> Result<()> {
    let pool = setup_test_db().await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());
    let def = setup_entity_type(&pool.pool, "grfv_bad").await?;

    let entity = make_entity(&def, "SomeEntity", 1);
    let uuid = repo.create(&entity).await?;

    let result = repo
        .get_raw_field_value(&def.entity_type, &uuid, "nonexistent_column")
        .await;
    assert!(result.is_err());
    Ok(())
}

// ── get_by_uuid_any_type (via trait) ──────────────────────────────────────────

#[tokio::test]
async fn test_get_by_uuid_any_type_finds_entity() -> Result<()> {
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));
    let def = setup_entity_type(&pool.pool, "gbuat_found").await?;

    let entity = make_entity(&def, "AnyTypeEntity", 99);
    let uuid = repo.create(&entity).await?;

    let found = repo.get_by_uuid_any_type(&uuid).await?;
    assert!(found.is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_by_uuid_any_type_returns_none_for_unknown() -> Result<()> {
    let pool = setup_test_db().await;
    let repo: Box<dyn DynamicEntityRepositoryTrait> =
        Box::new(DynamicEntityRepository::new(pool.pool.clone()));

    let found = repo.get_by_uuid_any_type(&Uuid::now_v7()).await?;
    assert!(found.is_none());
    Ok(())
}
