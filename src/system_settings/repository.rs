use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::system_settings::keys::SystemSettingKey;

pub struct SystemSettingsRepository {
    pool: PgPool,
}

impl SystemSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_value(&self, key: SystemSettingKey) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            r#"SELECT value FROM system_settings WHERE key = $1"#,
        )
        .bind(key.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(row.and_then(|r| r.try_get::<serde_json::Value, _>("value").ok()))
    }

    pub async fn upsert_value(
        &self,
        key: SystemSettingKey,
        value: &serde_json::Value,
        updated_by: Uuid,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO system_settings (key, value, updated_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (key) DO UPDATE
            SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = NOW()
            "#,
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
