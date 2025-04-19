// Basic workflow module - to be expanded in future
use crate::error::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Basic workflow state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Workflow is pending
    Pending,
    /// Workflow is in progress
    InProgress,
    /// Workflow is completed
    Completed,
    /// Workflow is rejected
    Rejected,
    /// Workflow is cancelled
    Cancelled,
    /// Custom state
    Custom(String),
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// UUID of the workflow definition
    pub uuid: Uuid,
    /// Name of the workflow
    pub name: String,
    /// Description of the workflow
    pub description: Option<String>,
    /// Entity type this workflow applies to
    pub entity_type: String,
    /// Version of the workflow
    pub version: i32,
    /// Whether this workflow is active
    pub active: bool,
    /// The workflow definition as JSON
    pub definition: serde_json::Value,
}

/// Workflow instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    /// UUID of the workflow instance
    pub uuid: Uuid,
    /// UUID of the workflow definition
    pub workflow_definition_uuid: Uuid,
    /// UUID of the entity this workflow is for
    pub entity_uuid: String,
    /// Type of the entity
    pub entity_type: String,
    /// Current state of the workflow
    pub state: serde_json::Value,
    /// Status of the workflow
    pub status: String,
    /// UUID of the user who created this workflow
    pub created_by: Uuid,
}

/// Workflow task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    /// UUID of the task
    pub uuid: Uuid,
    /// UUID of the workflow instance
    pub workflow_instance_uuid: Uuid,
    /// Name of the task
    pub task_name: String,
    /// Task definition
    pub task_definition: serde_json::Value,
    /// Status of the task
    pub status: String,
    /// Result of the task
    pub result: Option<serde_json::Value>,
    /// User assigned to the task
    pub assigned_to: Option<Uuid>,
    /// Due date of the task
    pub due_date: Option<OffsetDateTime>,
}

/// Workflow history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHistory {
    /// UUID of the history entry
    pub uuid: Uuid,
    /// UUID of the workflow instance
    pub workflow_instance_uuid: Uuid,
    /// Type of event
    pub event_type: String,
    /// Event data
    pub event_data: serde_json::Value,
    /// User who performed the action
    pub performed_by: Option<Uuid>,
    /// When the event occurred
    pub created_at: OffsetDateTime,
}

/// Workflow manager trait
pub trait WorkflowManager {
    /// Start a new workflow
    fn start_workflow(
        &self,
        definition_uuid: Uuid,
        entity_uuid: &str,
        entity_type: &str,
        user_uuid: Option<Uuid>,
    ) -> Result<WorkflowInstance>;

    /// Progress a workflow
    fn progress_workflow(
        &self,
        instance_uuid: Uuid,
        action: &str,
        data: &serde_json::Value,
        user_uuid: Option<Uuid>,
    ) -> Result<WorkflowInstance>;

    /// Complete a task
    fn complete_task(
        &self,
        task_uuid: Uuid,
        result: &serde_json::Value,
        user_uuid: Option<Uuid>,
    ) -> Result<WorkflowTask>;

    /// Get workflow tasks
    fn get_tasks(&self, instance_uuid: Uuid) -> Result<Vec<WorkflowTask>>;

    /// Get workflow history
    fn get_history(&self, instance_uuid: Uuid) -> Result<Vec<WorkflowHistory>>;
}
