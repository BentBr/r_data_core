use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::cache::CacheManager;
use crate::error::Result;
use crate::system_settings::entity_versioning::EntityVersioningSettings;
use crate::system_settings::keys::SystemSettingKey;
use crate::system_settings::repository::SystemSettingsRepository;

pub struct SettingsService {
    pub pool: PgPool,
    pub cache: Arc<CacheManager>,
}

impl SettingsService {
    pub fn new(pool: PgPool, cache: Arc<CacheManager>) -> Self {
        Self { pool, cache }
    }

    pub async fn get_entity_versioning_settings(&self) -> Result<EntityVersioningSettings> {
        let cache_key = SystemSettingKey::EntityVersioning.cache_key();
        if let Some(cached) = self
            .cache
            .get::<EntityVersioningSettings>(&cache_key)
            .await?
        {
            return Ok(cached);
        }

        let repo = SystemSettingsRepository::new(self.pool.clone());
        let settings: EntityVersioningSettings =
            match repo.get_value(SystemSettingKey::EntityVersioning).await? {
                Some(value) => {
                    serde_json::from_value::<EntityVersioningSettings>(value).unwrap_or_default()
                }
                None => EntityVersioningSettings::default(),
            };

        // cache
        let _ = self
            .cache
            .set(&cache_key, &settings, None)
            .await
            .map_err(|e| {
                log::warn!("Failed to cache settings: {}", e);
                e
            });

        Ok(settings)
    }

    pub async fn update_entity_versioning_settings(
        &self,
        new_settings: &EntityVersioningSettings,
        updated_by: Uuid,
    ) -> Result<()> {
        let json = serde_json::to_value(new_settings)?;
        let repo = SystemSettingsRepository::new(self.pool.clone());
        repo.upsert_value(SystemSettingKey::EntityVersioning, &json, updated_by)
            .await?;

        // Invalidate cache
        let _ = self
            .cache
            .delete(&SystemSettingKey::EntityVersioning.cache_key())
            .await;
        Ok(())
    }
}
