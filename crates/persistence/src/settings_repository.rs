#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::{PgPool, Row};
use uuid::Uuid;

use r_data_core_core::error::{Error, Result};
use r_data_core_core::settings::SystemSettingKey;

/// Repository for system settings storage
pub struct SystemSettingsRepository {
    pool: PgPool,
}

impl SystemSettingsRepository {
    /// Create a new system settings repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // PgPool is not const
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a setting value by key
    ///
    /// # Arguments
    /// * `key` - The setting key to retrieve
    ///
    /// # Returns
    /// The setting value as JSON, or `None` if not found
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_value(&self, key: SystemSettingKey) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query("SELECT value FROM system_settings WHERE key = $1")
            .bind(key.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Database)?;
        Ok(row.and_then(|r| r.try_get::<serde_json::Value, _>("value").ok()))
    }

    /// Insert or update a setting value
    ///
    /// # Arguments
    /// * `key` - The setting key
    /// * `value` - The setting value as JSON
    /// * `updated_by` - UUID of the user making the update
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn upsert_value(
        &self,
        key: SystemSettingKey,
        value: &serde_json::Value,
        updated_by: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "
            INSERT INTO system_settings (key, value, updated_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (key) DO UPDATE
            SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = NOW()
            ",
        )
        .bind(key.as_str())
        .bind(value)
        .bind(updated_by)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(())
    }
}
