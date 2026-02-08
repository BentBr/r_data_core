#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::settings::{
    EntityVersioningSettings, SystemSettingKey, WorkflowRunLogSettings,
};
use r_data_core_persistence::SystemSettingsRepository;

/// Service for managing system settings with caching
pub struct SettingsService {
    /// Database connection pool
    pub pool: PgPool,
    /// Cache manager for settings
    pub cache: Arc<CacheManager>,
}

impl SettingsService {
    /// Create a new settings service
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache` - Cache manager
    #[must_use]
    pub const fn new(pool: PgPool, cache: Arc<CacheManager>) -> Self {
        Self { pool, cache }
    }

    /// Get entity versioning settings with caching
    ///
    /// # Errors
    /// Returns an error if database query fails or cache operation fails
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
        let settings: EntityVersioningSettings = repo
            .get_value(SystemSettingKey::EntityVersioning)
            .await?
            .map_or_else(EntityVersioningSettings::default, |value| {
                serde_json::from_value::<EntityVersioningSettings>(value).unwrap_or_default()
            });

        // Cache the settings
        let _ = self
            .cache
            .set(&cache_key, &settings, None)
            .await
            .map_err(|e| {
                log::warn!("Failed to cache settings: {e}");
                e
            });

        Ok(settings)
    }

    /// Get workflow run log settings with caching
    ///
    /// # Errors
    /// Returns an error if database query fails or cache operation fails
    pub async fn get_workflow_run_log_settings(&self) -> Result<WorkflowRunLogSettings> {
        let cache_key = SystemSettingKey::WorkflowRunLogs.cache_key();
        if let Some(cached) = self.cache.get::<WorkflowRunLogSettings>(&cache_key).await? {
            return Ok(cached);
        }

        let repo = SystemSettingsRepository::new(self.pool.clone());
        let settings: WorkflowRunLogSettings = repo
            .get_value(SystemSettingKey::WorkflowRunLogs)
            .await?
            .map_or_else(WorkflowRunLogSettings::default, |value| {
                serde_json::from_value::<WorkflowRunLogSettings>(value).unwrap_or_default()
            });

        // Cache the settings
        let _ = self
            .cache
            .set(&cache_key, &settings, None)
            .await
            .map_err(|e| {
                log::warn!("Failed to cache settings: {e}");
                e
            });

        Ok(settings)
    }

    /// Update workflow run log settings
    ///
    /// # Arguments
    /// * `new_settings` - New workflow run log settings
    /// * `updated_by` - UUID of user updating the settings
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update_workflow_run_log_settings(
        &self,
        new_settings: &WorkflowRunLogSettings,
        updated_by: Uuid,
    ) -> Result<()> {
        let json = serde_json::to_value(new_settings)?;
        let repo = SystemSettingsRepository::new(self.pool.clone());
        repo.upsert_value(SystemSettingKey::WorkflowRunLogs, &json, updated_by)
            .await?;

        // Invalidate cache
        let _ = self
            .cache
            .delete(&SystemSettingKey::WorkflowRunLogs.cache_key())
            .await;
        Ok(())
    }

    /// Update entity versioning settings
    ///
    /// # Arguments
    /// * `new_settings` - New versioning settings
    /// * `updated_by` - UUID of user updating the settings
    ///
    /// # Errors
    /// Returns an error if database update fails
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
