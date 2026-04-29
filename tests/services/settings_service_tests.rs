#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::settings::{OutboxSettings, SystemSettingKey};
use r_data_core_persistence::SystemSettingsRepository;
use r_data_core_services::SettingsService;
use r_data_core_test_support::create_test_admin_user;

async fn maybe_setup_test_db() -> Option<r_data_core_test_support::TestDatabase> {
    let pool = r_data_core_test_support::try_setup_test_db().await;
    if pool.is_none() {
        eprintln!("Skipping settings service test: test database not available");
    }
    pool
}

fn create_cache_manager() -> Arc<CacheManager> {
    Arc::new(CacheManager::new(CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10_000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    }))
}

#[tokio::test]
async fn outbox_settings_defaults_when_missing() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };

    let service = SettingsService::new(pool.pool.clone(), create_cache_manager());
    let settings = service.get_outbox_settings().await?;
    assert!(!settings.fetch_enabled);
    assert!(!settings.push_enabled);
    Ok(())
}

#[tokio::test]
async fn outbox_settings_are_cached_and_update_invalidates_cache() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let cache = create_cache_manager();
    let service = SettingsService::new(pool.pool.clone(), cache.clone());
    let repo = SystemSettingsRepository::new(pool.pool.clone());
    let user_uuid = create_test_admin_user(&pool).await?;

    let seeded = OutboxSettings {
        fetch_enabled: false,
        push_enabled: true,
    };
    repo.upsert_value(
        SystemSettingKey::Outbox,
        &serde_json::to_value(&seeded)?,
        user_uuid,
    )
    .await?;

    let first_read = service.get_outbox_settings().await?;
    assert!(!first_read.fetch_enabled);
    assert!(first_read.push_enabled);

    let changed_in_db = OutboxSettings {
        fetch_enabled: true,
        push_enabled: false,
    };
    repo.upsert_value(
        SystemSettingKey::Outbox,
        &serde_json::to_value(&changed_in_db)?,
        user_uuid,
    )
    .await?;

    let cached_read = service.get_outbox_settings().await?;
    assert_eq!(cached_read.fetch_enabled, first_read.fetch_enabled);
    assert_eq!(cached_read.push_enabled, first_read.push_enabled);

    service
        .update_outbox_settings(
            &OutboxSettings {
                fetch_enabled: true,
                push_enabled: false,
            },
            user_uuid,
        )
        .await?;
    let refreshed = service.get_outbox_settings().await?;
    assert!(refreshed.fetch_enabled);
    assert!(!refreshed.push_enabled);

    let _ = cache.delete(&SystemSettingKey::Outbox.cache_key()).await;
    Ok(())
}
