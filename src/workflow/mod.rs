// Basic workflow module - to be expanded in future
use crate::error::Result;
use serde::{Deserialize, Serialize};
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
    /// ID of the workflow definition
    pub id: Uuid,
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
    /// ID of the workflow instance
    pub id: Uuid,
    /// ID of the workflow definition
    pub workflow_definition_id: Uuid,
    /// ID of the entity this workflow is for
    pub entity_id: String,
    /// Type of the entity
    pub entity_type: String,
    /// Current state of the workflow
    pub state: serde_json::Value,
    /// Status of the workflow
    pub status: String,
    /// ID of the user who created this workflow
    pub created_by: Option<Uuid>,
}

/// Workflow task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    /// ID of the task
    pub id: Uuid,
    /// ID of the workflow instance
    pub workflow_instance_id: Uuid,
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
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Workflow history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHistory {
    /// ID of the history entry
    pub id: Uuid,
    /// ID of the workflow instance
    pub workflow_instance_id: Uuid,
    /// Type of event
    pub event_type: String,
    /// Event data
    pub event_data: serde_json::Value,
    /// User who performed the action
    pub performed_by: Option<Uuid>,
    /// When the event occurred
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow manager trait
pub trait WorkflowManager {
    /// Start a new workflow
    fn start_workflow(
        &self,
        definition_id: Uuid,
        entity_id: &str,
        entity_type: &str,
        user_id: Option<Uuid>,
    ) -> Result<WorkflowInstance>;

    /// Progress a workflow
    fn progress_workflow(
        &self,
        instance_id: Uuid,
        action: &str,
        data: &serde_json::Value,
        user_id: Option<Uuid>,
    ) -> Result<WorkflowInstance>;

    /// Complete a task
    fn complete_task(
        &self,
        task_id: Uuid,
        result: &serde_json::Value,
        user_id: Option<Uuid>,
    ) -> Result<WorkflowTask>;

    /// Get workflow tasks
    fn get_tasks(&self, instance_id: Uuid) -> Result<Vec<WorkflowTask>>;

    /// Get workflow history
    fn get_history(&self, instance_id: Uuid) -> Result<Vec<WorkflowHistory>>;
}
