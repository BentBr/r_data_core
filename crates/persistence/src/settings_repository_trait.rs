#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::settings::SystemSettingKey;

/// Trait for system settings repository operations
#[async_trait]
pub trait SettingsRepositoryTrait: Send + Sync {
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
    async fn get_value(&self, key: SystemSettingKey) -> Result<Option<serde_json::Value>>;

    /// Insert or update a setting value
    ///
    /// # Arguments
    /// * `key` - The setting key
    /// * `value` - The setting value as JSON
    /// * `updated_by` - UUID of the user making the update
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn upsert_value(
        &self,
        key: SystemSettingKey,
        value: &serde_json::Value,
        updated_by: Uuid,
    ) -> Result<()>;
}
