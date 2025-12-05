#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use r_data_core_persistence::dashboard_stats_repository_trait::{
    DashboardStats as RepoDashboardStats, EntityStats as RepoEntityStats,
    EntityTypeCount as RepoEntityTypeCount, WorkflowStats as RepoWorkflowStats,
    WorkflowWithLatestStatus as RepoWorkflowWithLatestStatus,
};

/// Entity count for a specific type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityTypeCount {
    /// Entity type name
    pub entity_type: String,
    /// Count of entities of this type
    pub count: i64,
}

/// Entity statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityStats {
    /// Total count of all entities across all types
    pub total: i64,
    /// Breakdown by entity type
    pub by_type: Vec<EntityTypeCount>,
}

/// Workflow with its latest run status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowWithLatestStatus {
    /// Workflow UUID
    pub uuid: String,
    /// Workflow name
    pub name: String,
    /// Latest run status (if any runs exist)
    pub latest_status: Option<String>,
}

/// Workflow statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowStats {
    /// Total count of workflows
    pub total: i64,
    /// List of workflows with their latest run status
    pub workflows: Vec<WorkflowWithLatestStatus>,
}

/// Dashboard statistics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

// Conversion implementations from repository types to API model types
impl From<RepoEntityTypeCount> for EntityTypeCount {
    fn from(repo: RepoEntityTypeCount) -> Self {
        Self {
            entity_type: repo.entity_type,
            count: repo.count,
        }
    }
}

impl From<RepoEntityStats> for EntityStats {
    fn from(repo: RepoEntityStats) -> Self {
        Self {
            total: repo.total,
            by_type: repo.by_type.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RepoWorkflowWithLatestStatus> for WorkflowWithLatestStatus {
    fn from(repo: RepoWorkflowWithLatestStatus) -> Self {
        Self {
            uuid: repo.uuid,
            name: repo.name,
            latest_status: repo.latest_status,
        }
    }
}

impl From<RepoWorkflowStats> for WorkflowStats {
    fn from(repo: RepoWorkflowStats) -> Self {
        Self {
            total: repo.total,
            workflows: repo.workflows.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RepoDashboardStats> for DashboardStats {
    fn from(repo: RepoDashboardStats) -> Self {
        Self {
            entity_definitions_count: repo.entity_definitions_count,
            entities: repo.entities.into(),
            workflows: repo.workflows.into(),
            online_users_count: repo.online_users_count,
        }
    }
}
