#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;

use r_data_core_core::error::Result;

/// Entity count by type
#[derive(Debug, Clone)]
pub struct EntityTypeCount {
    /// Entity type name
    pub entity_type: String,
    /// Count of entities of this type
    pub count: i64,
}

/// Entity statistics
#[derive(Debug, Clone)]
pub struct EntityStats {
    /// Total count of all entities across all types
    pub total: i64,
    /// Breakdown by entity type
    pub by_type: Vec<EntityTypeCount>,
}

/// Workflow with its latest run status
#[derive(Debug, Clone)]
pub struct WorkflowWithLatestStatus {
    /// Workflow UUID
    pub uuid: String,
    /// Workflow name
    pub name: String,
    /// Latest run status (if any runs exist)
    pub latest_status: Option<String>,
}

/// Workflow statistics
#[derive(Debug, Clone)]
pub struct WorkflowStats {
    /// Total count of workflows
    pub total: i64,
    /// List of workflows with their latest run status
    pub workflows: Vec<WorkflowWithLatestStatus>,
}

/// Dashboard statistics
#[derive(Debug, Clone)]
pub struct DashboardStats {
    /// Total count of entity definitions
    pub entity_definitions_count: i64,
    /// Entity statistics
    pub entities: EntityStats,
    /// Workflow statistics
    pub workflows: WorkflowStats,
    /// Count of online users (users with active refresh tokens)
    pub online_users_count: i64,
}

/// Trait for dashboard statistics repository operations
#[async_trait]
pub trait DashboardStatsRepositoryTrait: Send + Sync {
    /// Get dashboard statistics
    ///
    /// # Errors
    /// Returns an error if database queries fail
    async fn get_dashboard_stats(&self) -> Result<DashboardStats>;
}
