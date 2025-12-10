#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use r_data_core_workflow::data::{
    requests::{CreateWorkflowRequest, UpdateWorkflowRequest},
    Workflow,
};

/// Trait for workflow repository operations
#[async_trait]
pub trait WorkflowRepositoryTrait: Send + Sync {
    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_all(&self) -> anyhow::Result<Vec<Workflow>>;

    /// List workflows with pagination and sorting
    ///
    /// # Arguments
    /// * `limit` - Maximum number of workflows to return (-1 for unlimited)
    /// * `offset` - Number of workflows to skip
    /// * `sort_by` - Optional field to sort by
    /// * `sort_order` - Sort order (ASC or DESC), defaults to ASC
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> anyhow::Result<Vec<Workflow>>;

    /// Count all workflows
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn count_all(&self) -> anyhow::Result<i64>;

    /// Get workflow by UUID
    ///
    /// # Arguments
    /// * `uuid` - Workflow UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_by_uuid(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>>;

    /// Create a new workflow
    ///
    /// # Arguments
    /// * `req` - Create workflow request
    /// * `created_by` - UUID of user creating the workflow
    ///
    /// # Errors
    /// Returns an error if creation fails
    async fn create(&self, req: &CreateWorkflowRequest, created_by: Uuid) -> anyhow::Result<Uuid>;

    /// Update an existing workflow
    ///
    /// # Arguments
    /// * `uuid` - Workflow UUID
    /// * `req` - Update workflow request
    /// * `updated_by` - UUID of user updating the workflow
    ///
    /// # Errors
    /// Returns an error if update fails
    async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> anyhow::Result<()>;

    /// Delete a workflow
    ///
    /// # Arguments
    /// * `uuid` - Workflow UUID
    ///
    /// # Errors
    /// Returns an error if deletion fails
    async fn delete(&self, uuid: Uuid) -> anyhow::Result<()>;

    /// List scheduled consumer workflows
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>>;

    /// Mark a run as running (transition queued -> running)
    async fn mark_run_running(&self, run_uuid: Uuid) -> anyhow::Result<()>;

    /// Mark a run as successful
    async fn mark_run_success(
        &self,
        run_uuid: Uuid,
        processed: i64,
        failed: i64,
    ) -> anyhow::Result<()>;

    /// Mark a run as failed
    async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> anyhow::Result<()>;

    /// Get run status
    async fn get_run_status(&self, run_uuid: Uuid) -> anyhow::Result<Option<String>>;

    /// Insert a new workflow run in queued status
    ///
    /// # Arguments
    /// * `workflow_uuid` - Workflow UUID
    /// * `trigger_id` - Trigger UUID
    ///
    /// # Errors
    /// Returns an error if insertion fails
    async fn insert_run_queued(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Uuid,
    ) -> anyhow::Result<Uuid>;

    /// List workflow runs with pagination
    ///
    /// # Arguments
    /// * `workflow_uuid` - Workflow UUID
    /// * `limit` - Maximum number of runs to return
    /// * `offset` - Number of runs to skip
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )>;

    /// List run logs with pagination
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    /// * `limit` - Maximum number of logs to return
    /// * `offset` - Number of logs to skip
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(Uuid, String, String, String, Option<serde_json::Value>)>,
        i64,
    )>;

    /// Check if a run exists
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool>;

    /// List all runs with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of runs to return
    /// * `offset` - Number of runs to skip
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )>;

    /// Insert a run log entry
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    /// * `level` - Log level
    /// * `message` - Log message
    /// * `meta` - Optional metadata
    ///
    /// # Errors
    /// Returns an error if insertion fails
    async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> anyhow::Result<()>;

    /// Insert raw items for a workflow run
    ///
    /// # Arguments
    /// * `workflow_uuid` - Workflow UUID
    /// * `run_uuid` - Run UUID
    /// * `payloads` - Vector of payload JSON values
    ///
    /// # Errors
    /// Returns an error if insertion fails
    async fn insert_raw_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> anyhow::Result<i64>;

    /// Count raw items for a run
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> anyhow::Result<i64>;

    /// Mark raw items as processed
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    ///
    /// # Errors
    /// Returns an error if update fails
    async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> anyhow::Result<()>;

    /// Fetch staged raw items for processing
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    /// * `limit` - Maximum number of items to fetch
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<(Uuid, serde_json::Value)>>;

    /// Set raw item status
    ///
    /// # Arguments
    /// * `item_uuid` - Item UUID
    /// * `status` - Status string
    /// * `error` - Optional error message
    ///
    /// # Errors
    /// Returns an error if update fails
    async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Get workflow UUID for a run
    ///
    /// # Arguments
    /// * `run_uuid` - Run UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_workflow_uuid_for_run(&self, run_uuid: Uuid) -> anyhow::Result<Option<Uuid>>;
}
