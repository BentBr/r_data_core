#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::workflow_versioning_repository::{WorkflowVersionMeta, WorkflowVersionPayload};
use r_data_core_core::error::Result;

/// Trait for workflow versioning repository operations
#[async_trait]
pub trait WorkflowVersioningRepositoryTrait: Send + Sync {
    /// Create a pre-update snapshot for a workflow
    /// The snapshot's `created_by` is extracted from the JSON data (`updated_by` or `created_by`).
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn snapshot_pre_update(&self, workflow_uuid: Uuid) -> Result<()>;

    /// List all versions for a workflow
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_workflow_versions(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Vec<WorkflowVersionMeta>>;

    /// Get a specific version of a workflow
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    /// * `version_number` - Version number to retrieve
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_workflow_version(
        &self,
        workflow_uuid: Uuid,
        version_number: i32,
    ) -> Result<Option<WorkflowVersionPayload>>;

    /// Get current workflow metadata
    ///
    /// # Arguments
    /// * `workflow_uuid` - UUID of the workflow
    ///
    /// # Returns
    /// Tuple of (`version`, `updated_at`, `updated_by`, `updated_by_name`)
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_current_workflow_metadata(
        &self,
        workflow_uuid: Uuid,
    ) -> Result<Option<(i32, OffsetDateTime, Option<Uuid>, Option<String>)>>;

    /// Prune workflow versions older than the specified number of days
    ///
    /// # Arguments
    /// * `days` - Number of days to keep
    ///
    /// # Returns
    /// Number of versions deleted
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn prune_older_than_days(&self, days: i32) -> Result<u64>;

    /// Prune workflow versions, keeping only the latest N versions per workflow
    ///
    /// # Arguments
    /// * `keep` - Number of latest versions to keep per workflow
    ///
    /// # Returns
    /// Number of versions deleted
    ///
    /// # Errors
    /// Returns an error if database operation fails
    async fn prune_keep_latest_per_workflow(&self, keep: i32) -> Result<u64>;
}

