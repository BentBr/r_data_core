#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use r_data_core_core::error::Result;

/// Entity definitions statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinitionsStats {
    /// Total count
    pub count: i64,
    /// List of entity definition names
    pub names: Vec<String>,
}

/// Entity count per definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCount {
    /// Entity definition name
    pub entity_type: String,
    /// Count of entities for this definition
    pub count: i64,
}

/// Trait for statistics repository operations
#[async_trait]
pub trait StatisticsRepositoryTrait: Send + Sync {
    /// Get entity definitions statistics
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_entity_definitions_stats(&self) -> Result<EntityDefinitionsStats>;

    /// Get entities count per definition
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_entities_per_definition(&self) -> Result<Vec<EntityCount>>;

    /// Get users count
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_users_count(&self) -> Result<i64>;

    /// Get roles count
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_roles_count(&self) -> Result<i64>;

    /// Get API keys count
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_api_keys_count(&self) -> Result<i64>;

    /// Get workflows count
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_workflows_count(&self) -> Result<i64>;

    /// Get workflow logs count
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_workflow_logs_count(&self) -> Result<i64>;
}
