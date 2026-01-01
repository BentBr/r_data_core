#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;

use crate::statistics_repository_trait::{
    EntityCount, EntityDefinitionsStats, StatisticsRepositoryTrait,
};
use r_data_core_core::error::Result;

/// Repository for statistics collection
pub struct StatisticsRepository {
    pool: PgPool,
}

impl StatisticsRepository {
    /// Create a new statistics repository
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StatisticsRepositoryTrait for StatisticsRepository {
    async fn get_entity_definitions_stats(&self) -> Result<EntityDefinitionsStats> {
        let rows = sqlx::query!("SELECT entity_type FROM entity_definitions ORDER BY entity_type")
            .fetch_all(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        let names: Vec<String> = rows.into_iter().map(|row| row.entity_type).collect();

        #[allow(clippy::cast_possible_wrap)] // Entity definition count will never exceed i64::MAX
        Ok(EntityDefinitionsStats {
            count: names.len() as i64,
            names,
        })
    }

    async fn get_entities_per_definition(&self) -> Result<Vec<EntityCount>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                entity_type,
                COUNT(*) as count
            FROM entities_registry
            GROUP BY entity_type
            ORDER BY entity_type
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(rows
            .into_iter()
            .map(|row| EntityCount {
                entity_type: row.entity_type,
                count: row.count.unwrap_or(0),
            })
            .collect())
    }

    async fn get_users_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM admin_users")
            .fetch_one(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(count.unwrap_or(0))
    }

    async fn get_roles_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM roles")
            .fetch_one(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(count.unwrap_or(0))
    }

    async fn get_api_keys_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(count.unwrap_or(0))
    }

    async fn get_workflows_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM workflows")
            .fetch_one(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(count.unwrap_or(0))
    }

    async fn get_workflow_logs_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM workflow_runs")
            .fetch_one(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(count.unwrap_or(0))
    }
}
