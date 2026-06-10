#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::settings::{
    EntityVersioningSettings, OutboxSettings, SystemSettingKey, WorkflowRunLogSettings,
};
use r_data_core_persistence::SystemSettingsRepository;

/// Default cache TTL for mutable system settings, in seconds.
///
/// System settings are toggled at runtime via the admin API but consumed by
/// other processes (e.g. the workflow worker). The cache is two-tier: a shared
/// Redis layer plus a per-process in-memory layer. An update only invalidates
/// the writer's in-memory layer and the shared Redis entry — other processes'
/// in-memory entries are never told to drop. A short TTL bounds how long those
/// processes can serve a stale value to roughly this many seconds, instead of
/// the much longer default cache TTL.
pub const SETTINGS_CACHE_TTL_SECS: u64 = 10;

/// Service for managing system settings with caching
pub struct SettingsService {
    /// Database connection pool
    pub pool: PgPool,
    /// Cache manager for settings
    pub cache: Arc<CacheManager>,
    /// Default outbox settings when no value is stored in DB.
    pub outbox_defaults: OutboxSettings,
    /// TTL applied when caching settings, in seconds. Kept short so runtime
    /// changes propagate across processes promptly. See [`SETTINGS_CACHE_TTL_SECS`].
    pub settings_cache_ttl_secs: u64,
}

impl SettingsService {
    /// Create a new settings service
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache` - Cache manager
    #[must_use]
    pub const fn new(pool: PgPool, cache: Arc<CacheManager>) -> Self {
        Self {
            pool,
            cache,
            outbox_defaults: OutboxSettings {
                fetch_enabled: false,
                push_enabled: false,
            },
            settings_cache_ttl_secs: SETTINGS_CACHE_TTL_SECS,
        }
    }

    /// Override outbox defaults used when DB value is missing.
    #[must_use]
    pub const fn with_outbox_defaults(mut self, outbox_defaults: OutboxSettings) -> Self {
        self.outbox_defaults = outbox_defaults;
        self
    }

    /// Override the TTL used when caching settings, in seconds.
    ///
    /// Production uses [`SETTINGS_CACHE_TTL_SECS`]; this exists mainly so tests
    /// can exercise cache expiry without long sleeps.
    #[must_use]
    pub const fn with_settings_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.settings_cache_ttl_secs = ttl_secs;
        self
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
            .set(&cache_key, &settings, Some(self.settings_cache_ttl_secs))
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
            .set(&cache_key, &settings, Some(self.settings_cache_ttl_secs))
            .await
            .map_err(|e| {
                log::warn!("Failed to cache settings: {e}");
                e
            });

        Ok(settings)
    }

    /// Get outbox settings with caching
    ///
    /// # Errors
    /// Returns an error if database query fails or cache operation fails
    pub async fn get_outbox_settings(&self) -> Result<OutboxSettings> {
        let cache_key = SystemSettingKey::Outbox.cache_key();
        if let Some(cached) = self.cache.get::<OutboxSettings>(&cache_key).await? {
            return Ok(cached);
        }

        let repo = SystemSettingsRepository::new(self.pool.clone());
        let settings: OutboxSettings = repo.get_value(SystemSettingKey::Outbox).await?.map_or_else(
            || self.outbox_defaults.clone(),
            |value| serde_json::from_value::<OutboxSettings>(value).unwrap_or_default(),
        );

        let _ = self
            .cache
            .set(&cache_key, &settings, Some(self.settings_cache_ttl_secs))
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

    /// Update outbox settings
    ///
    /// # Arguments
    /// * `new_settings` - New outbox settings
    /// * `updated_by` - UUID of user updating the settings
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update_outbox_settings(
        &self,
        new_settings: &OutboxSettings,
        updated_by: Uuid,
    ) -> Result<()> {
        let json = serde_json::to_value(new_settings)?;
        let repo = SystemSettingsRepository::new(self.pool.clone());
        repo.upsert_value(SystemSettingKey::Outbox, &json, updated_by)
            .await?;

        let _ = self
            .cache
            .delete(&SystemSettingKey::Outbox.cache_key())
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
