#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_core::settings::EntityVersioningSettings;
use r_data_core_services::SettingsService;
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::version_purger::VersionPurgerTask;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_version_purger_task_name_and_cron() -> Result<()> {
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());

    assert_eq!(task.name(), "version_purger");
    assert_eq!(task.cron(), "0 0 * * * *");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_version_purger_task_skips_when_disabled() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;

    // Create cache manager and settings service
    let cache_config = CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Disable versioning
    let settings = EntityVersioningSettings {
        enabled: false,
        max_versions: Some(5),
        max_age_days: Some(30),
    };
    settings_service
        .update_entity_versioning_settings(&settings, user_uuid)
        .await?;

    // Execute the task - should skip when disabled
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_version_purger_task_prunes_by_age() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let entity_uuid = Uuid::now_v7();

    // Create cache manager and settings service
    let cache_config = CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Enable versioning with age-based pruning (30 days)
    let settings = EntityVersioningSettings {
        enabled: true,
        max_versions: None,
        max_age_days: Some(30),
    };
    settings_service
        .update_entity_versioning_settings(&settings, user_uuid)
        .await?;

    // Create a dummy entity in entities_registry to satisfy foreign key constraint
    sqlx::query(
        "INSERT INTO entities_registry (uuid, entity_type, path, entity_key, version, created_at, updated_at, created_by, published)
         VALUES ($1, $2, '/', $1::text, 3, NOW(), NOW(), $3, true)
         ON CONFLICT (uuid) DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(user_uuid)
    .execute(&pool.pool)
    .await?;

    // Create versions: one old (40 days), one recent (10 days)
    sqlx::query(
        "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at)
         VALUES ($1, $2, 1, $3, NOW() - INTERVAL '40 days')
         ON CONFLICT DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(serde_json::json!({"v": 1}))
    .execute(&pool.pool)
    .await?;

    sqlx::query(
        "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at)
         VALUES ($1, $2, 2, $3, NOW() - INTERVAL '10 days')
         ON CONFLICT DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(serde_json::json!({"v": 2}))
    .execute(&pool.pool)
    .await?;

    // Verify both versions exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 2, "Should have 2 versions before pruning");

    // Execute the task
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify old version is removed, recent version remains
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        count_after, 1,
        "Should have 1 version after age-based pruning"
    );

    // Verify the remaining version is the recent one
    let remaining_version: i32 =
        sqlx::query_scalar("SELECT version_number FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        remaining_version, 2,
        "Remaining version should be version 2"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_version_purger_task_prunes_by_count() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let entity_uuid = Uuid::now_v7();

    // Create cache manager and settings service
    let cache_config = CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Enable versioning with count-based pruning (keep latest 2)
    let settings = EntityVersioningSettings {
        enabled: true,
        max_versions: Some(2),
        max_age_days: None,
    };
    settings_service
        .update_entity_versioning_settings(&settings, user_uuid)
        .await?;

    // Create a dummy entity in entities_registry
    sqlx::query(
        "INSERT INTO entities_registry (uuid, entity_type, path, entity_key, version, created_at, updated_at, created_by, published)
         VALUES ($1, $2, '/', $1::text, 5, NOW(), NOW(), $3, true)
         ON CONFLICT (uuid) DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(user_uuid)
    .execute(&pool.pool)
    .await?;

    // Create 5 versions
    for v in 1..=5 {
        sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at)
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT DO NOTHING",
        )
        .bind(entity_uuid)
        .bind("dynamic_entity")
        .bind(v)
        .bind(serde_json::json!({"v": v}))
        .execute(&pool.pool)
        .await?;
    }

    // Verify all 5 versions exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 5, "Should have 5 versions before pruning");

    // Execute the task
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify only 2 versions remain (latest 2)
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        count_after, 2,
        "Should have 2 versions after count-based pruning"
    );

    // Verify the remaining versions are the latest ones (4 and 5)
    let remaining_versions: Vec<i32> = sqlx::query_scalar(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number",
    )
    .bind(entity_uuid)
    .fetch_all(&pool.pool)
    .await?;
    assert_eq!(
        remaining_versions,
        vec![4, 5],
        "Should keep latest 2 versions"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_version_purger_task_prunes_by_both_age_and_count() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let entity_uuid = Uuid::now_v7();

    // Create cache manager and settings service
    let cache_config = CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Enable versioning with both age and count pruning
    let settings = EntityVersioningSettings {
        enabled: true,
        max_versions: Some(3),
        max_age_days: Some(30),
    };
    settings_service
        .update_entity_versioning_settings(&settings, user_uuid)
        .await?;

    // Create a dummy entity in entities_registry
    sqlx::query(
        "INSERT INTO entities_registry (uuid, entity_type, path, entity_key, version, created_at, updated_at, created_by, published)
         VALUES ($1, $2, '/', $1::text, 5, NOW(), NOW(), $3, true)
         ON CONFLICT (uuid) DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(user_uuid)
    .execute(&pool.pool)
    .await?;

    // Create versions: 2 old (40 days), 3 recent (10 days)
    for v in 1..=2 {
        sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at)
             VALUES ($1, $2, $3, $4, NOW() - INTERVAL '40 days')
             ON CONFLICT DO NOTHING",
        )
        .bind(entity_uuid)
        .bind("dynamic_entity")
        .bind(v)
        .bind(serde_json::json!({"v": v}))
        .execute(&pool.pool)
        .await?;
    }

    for v in 3..=5 {
        sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at)
             VALUES ($1, $2, $3, $4, NOW() - INTERVAL '10 days')
             ON CONFLICT DO NOTHING",
        )
        .bind(entity_uuid)
        .bind("dynamic_entity")
        .bind(v)
        .bind(serde_json::json!({"v": v}))
        .execute(&pool.pool)
        .await?;
    }

    // Verify all 5 versions exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 5, "Should have 5 versions before pruning");

    // Execute the task
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // After age pruning: 2 old versions removed, 3 recent remain
    // After count pruning: keep latest 3, but we already have 3, so all should remain
    // Actually, age pruning happens first, so we should have 3 remaining after age pruning
    // Then count pruning should keep latest 3, which are all we have
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_versions WHERE entity_uuid = $1")
            .bind(entity_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        count_after, 3,
        "Should have 3 versions after both pruning operations"
    );

    // Verify the remaining versions are the recent ones (3, 4, 5)
    let remaining_versions: Vec<i32> = sqlx::query_scalar(
        "SELECT version_number FROM entities_versions WHERE entity_uuid = $1 ORDER BY version_number",
    )
    .bind(entity_uuid)
    .fetch_all(&pool.pool)
    .await?;
    assert_eq!(
        remaining_versions,
        vec![3, 4, 5],
        "Should keep latest 3 recent versions"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_version_purger_task_with_no_versions() -> Result<()> {
    // Setup test database
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;

    // Create cache manager and settings service
    let cache_config = CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Enable versioning
    let settings = EntityVersioningSettings {
        enabled: true,
        max_versions: Some(5),
        max_age_days: Some(30),
    };
    settings_service
        .update_entity_versioning_settings(&settings, user_uuid)
        .await?;

    // Execute the task with no versions
    let task = VersionPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);

    // Should not error when there are no versions to prune
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    Ok(())
}
