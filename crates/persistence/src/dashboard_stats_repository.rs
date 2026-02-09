#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;

use crate::dashboard_stats_repository_trait::{
    DashboardStats, DashboardStatsRepositoryTrait, EntityStats, EntityTypeCount, WorkflowStats,
    WorkflowWithLatestStatus,
};
use r_data_core_core::error::Result;

/// Repository for dashboard statistics
pub struct DashboardStatsRepository {
    pool: PgPool,
}

impl DashboardStatsRepository {
    /// Create a new dashboard stats repository
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl DashboardStatsRepositoryTrait for DashboardStatsRepository {
    async fn get_dashboard_stats(&self) -> Result<DashboardStats> {
        // Fetch all stats in parallel for better performance
        let (entity_defs_count, entity_stats, workflow_stats, online_users_count) = tokio::join!(
            get_entity_definitions_count(&self.pool),
            get_entity_stats(&self.pool),
            get_workflow_stats(&self.pool),
            get_online_users_count(&self.pool)
        );

        Ok(DashboardStats {
            entity_definitions_count: entity_defs_count?,
            entities: entity_stats?,
            workflows: workflow_stats?,
            online_users_count: online_users_count?,
        })
    }
}

/// Get entity definitions count
async fn get_entity_definitions_count(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar!("SELECT COUNT(*) as count FROM entity_definitions")
        .fetch_one(pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    Ok(count.unwrap_or(0))
}

/// Get entity statistics (total and by type)
async fn get_entity_stats(pool: &PgPool) -> Result<EntityStats> {
    // Single query to get counts by type and total
    let rows = sqlx::query!(
        r#"
        SELECT 
            entity_type,
            COUNT(*) as count
        FROM entities_registry
        GROUP BY entity_type
        ORDER BY count DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    let mut by_type = Vec::new();
    let mut total = 0i64;

    for row in rows {
        if let Some(count) = row.count {
            total += count;
            by_type.push(EntityTypeCount {
                entity_type: row.entity_type,
                count,
            });
        }
    }

    Ok(EntityStats { total, by_type })
}

/// Get workflow statistics (count and latest run statuses)
async fn get_workflow_stats(pool: &PgPool) -> Result<WorkflowStats> {
    // Single query using DISTINCT ON to get latest run status for each workflow
    let rows = sqlx::query!(
        r#"
        WITH latest_runs AS (
            SELECT DISTINCT ON (workflow_uuid)
                workflow_uuid,
                status::text as status
            FROM workflow_runs
            ORDER BY workflow_uuid, queued_at DESC
        )
        SELECT 
            w.uuid,
            w.name,
            lr.status as latest_status
        FROM workflows w
        LEFT JOIN latest_runs lr ON w.uuid = lr.workflow_uuid
        ORDER BY w.name
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    let total = i64::try_from(rows.len()).unwrap_or(0);
    let workflows = rows
        .into_iter()
        .map(|row| WorkflowWithLatestStatus {
            uuid: row.uuid.to_string(),
            name: row.name,
            latest_status: row.latest_status,
        })
        .collect();

    Ok(WorkflowStats { total, workflows })
}

/// Get online users count (users with active refresh tokens)
async fn get_online_users_count(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(DISTINCT user_id) as count
        FROM refresh_tokens
        WHERE is_revoked = false AND expires_at > NOW()
        "#
    )
    .fetch_one(pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    Ok(count.unwrap_or(0))
}
