#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_persistence::DashboardStatsRepositoryTrait;
use std::sync::Arc;

/// Service for dashboard statistics
#[derive(Clone)]
pub struct DashboardStatsService {
    repository: Arc<dyn DashboardStatsRepositoryTrait>,
}

impl DashboardStatsService {
    /// Create a new dashboard stats service
    ///
    /// # Arguments
    /// * `repository` - Repository for dashboard statistics
    #[must_use]
    pub fn new(repository: Arc<dyn DashboardStatsRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Get dashboard statistics
    ///
    /// # Errors
    /// Returns an error if database queries fail
    pub async fn get_dashboard_stats(
        &self,
    ) -> r_data_core_core::error::Result<
        r_data_core_persistence::dashboard_stats_repository_trait::DashboardStats,
    > {
        self.repository.get_dashboard_stats().await
    }
}
