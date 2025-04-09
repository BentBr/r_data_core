use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgTypeInfo, PgValueRef};
use sqlx::{postgres::PgRow, Decode, FromRow, Row, Type};
use sqlx::encode::{Encode, IsNull};
use sqlx::postgres::PgArgumentBuffer;
use std::collections::HashMap;
use std::default::Default;
use utoipa::ToSchema;
use uuid::Uuid;

use super::AbstractRDataEntity;
use crate::error::{Error, Result};

/// Trigger types for starting a workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum TriggerType {
    /// Manually triggered
    Manual,

    /// Triggered by schedule (cron-like)
    Schedule(String),

    /// Triggered when an entity is created
    EntityCreated(String),

    /// Triggered when an entity is updated
    EntityUpdated(String),

    /// Triggered when an entity is deleted
    EntityDeleted(String),

    /// Triggered by HTTP webhook
    WebHook(String),
}

/// Implement Default for TriggerType
impl Default for TriggerType {
    fn default() -> Self {
        TriggerType::Manual
    }
}

/// Action types that can be performed in a workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum ActionType {
    /// Run a script
    Script(String),

    /// Make an HTTP request
    HttpRequest,

    /// Import data from a source
    Import,

    /// Export data to a destination
    Export,

    /// Transform data
    Transform,

    /// Send notification
    Notification,

    /// Create a new entity
    CreateEntity,

    /// Update an existing entity
    UpdateEntity,

    /// Delete an entity
    DeleteEntity,

    /// Custom action
    Custom(String),
}

/// A workflow step configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowStep {
    /// Step UUID
    pub uuid: Uuid,

    /// Step name
    pub name: String,

    /// Step description
    pub description: Option<String>,

    /// Action to perform
    pub action: ActionType,

    /// Step configuration as JSON
    pub config: serde_json::Value,

    /// Next step UUIDs (empty for terminal steps)
    pub next_steps: Vec<Uuid>,

    /// Condition for branching (optional)
    pub condition: Option<String>,
}

impl Type<sqlx::Postgres> for WorkflowStep {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("JSONB")
    }
}

// Add implementation of PgHasArrayType for WorkflowStep
impl sqlx::postgres::PgHasArrayType for WorkflowStep {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_JSONB")
    }
}

// Add Encode implementation to ensure WorkflowStep can be saved
impl Encode<'_, sqlx::Postgres> for WorkflowStep {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
        let json = serde_json::to_value(self).expect("Failed to serialize WorkflowStep to JSON");
        <serde_json::Value as Encode<sqlx::Postgres>>::encode(json, buf)
    }
    
    fn size_hint(&self) -> usize {
        4096  // Conservative size estimate
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for WorkflowStep {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let json = <serde_json::Value as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(serde_json::from_value(json)?)
    }
}

impl Type<sqlx::Postgres> for TriggerType {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("VARCHAR")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for TriggerType {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(serde_json::from_str(&s)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSteps(pub Vec<WorkflowStep>);

impl Type<sqlx::Postgres> for WorkflowSteps {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("JSONB")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for WorkflowSteps {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let json = <serde_json::Value as Decode<sqlx::Postgres>>::decode(value)?;
        Ok(WorkflowSteps(serde_json::from_value(json)?))
    }
}

/// Entity for defining a workflow
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowEntity {
    /// Base entity properties
    #[serde(flatten)]
    pub base: AbstractRDataEntity,

    /// Workflow name
    pub name: String,

    /// Workflow description
    pub description: Option<String>,

    /// What triggers this workflow
    pub trigger: TriggerType,

    /// The first step in the workflow
    pub first_step: Option<Uuid>,

    /// All steps in the workflow
    pub steps: WorkflowSteps,

    /// Whether the workflow is enabled
    pub enabled: bool,

    /// Timeout in seconds (0 = no timeout)
    pub timeout_seconds: i32,

    /// Max retry count for failures
    pub max_retries: i32,

    /// Whether to keep execution history
    pub keep_history: bool,
}

impl WorkflowEntity {
    /// Create a new workflow
    pub fn new(name: String, trigger: TriggerType) -> Self {
        Self {
            base: AbstractRDataEntity::new("/workflows".to_string()),
            name,
            description: None,
            trigger,
            first_step: None,
            steps: WorkflowSteps(Vec::new()),
            enabled: false,
            timeout_seconds: 300,
            max_retries: 3,
            keep_history: true,
        }
    }

    /// Add a step to the workflow
    pub fn add_step(&mut self, step: WorkflowStep) -> Result<()> {
        // Check if the step ID already exists
        if self.steps.0.iter().any(|s| s.uuid == step.uuid) {
            return Err(Error::Workflow(format!(
                "Step with UUID {} already exists",
                step.uuid
            )));
        }

        // Validate transitions against existing steps
        for transition in &step.next_steps {
            if !self.steps.0.iter().any(|s| s.uuid == *transition) && *transition != step.uuid {
                return Err(Error::Workflow(format!(
                    "Transition target step UUID {} does not exist",
                    *transition
                )));
            }
        }

        self.steps.0.push(step);
        Ok(())
    }

    /// Get a step by UUID
    pub fn get_step(&self, uuid: Uuid) -> Option<&WorkflowStep> {
        self.steps.0.iter().find(|s| s.uuid == uuid)
    }

    /// Remove a step from the workflow
    pub fn remove_step(&mut self, uuid: Uuid) -> Result<()> {
        // Check if any other steps have transitions to this step
        let step_idx = self.steps.0.iter().position(|s| s.uuid == uuid);

        if let Some(idx) = step_idx {
            // Check if any other steps reference this step in transitions
            for step in &self.steps.0 {
                if step.uuid != uuid && step.next_steps.iter().any(|t| t == &uuid) {
                    return Err(Error::Workflow(format!(
                        "Cannot remove step {} because step {} has a transition to it",
                        uuid, step.uuid
                    )));
                }
            }

            self.steps.0.remove(idx);
            Ok(())
        } else {
            Err(Error::Workflow(format!("Step with UUID {} not found", uuid)))
        }
    }

    /// Validate the workflow
    pub fn validate(&self) -> Result<()> {
        // Ensure we have at least one step
        if self.steps.0.is_empty() {
            return Err(Error::Workflow(
                "Workflow must have at least one step".to_string(),
            ));
        }

        // Ensure we have a valid initial step
        if let Some(first_uuid) = self.first_step {
            if !self.steps.0.iter().any(|s| s.uuid == first_uuid) {
                return Err(Error::Workflow(format!(
                    "Initial step UUID {} does not exist in steps",
                    first_uuid
                )));
            }
        }

        // Validate all transitions point to valid step UUIDs
        for step in &self.steps.0 {
            for &next_uuid in &step.next_steps {
                if !self.steps.0.iter().any(|s| s.uuid == next_uuid) {
                    return Err(Error::Workflow(format!(
                        "Transition in step {} points to non-existent step {}",
                        step.uuid, next_uuid
                    )));
                }
            }
        }

        Ok(())
    }
}

impl FromRow<'_, PgRow> for WorkflowEntity {
    fn from_row(row: &PgRow) -> std::result::Result<Self, sqlx::Error> {
        // Extract base entity fields
        let uuid = row.try_get::<Uuid, _>("uuid")?;

        let base = AbstractRDataEntity {
            uuid,
            path: row.try_get("path")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            created_by: row.try_get("created_by")?,
            updated_by: row.try_get("updated_by")?,
            published: row.try_get("published")?,
            version: row.try_get("version")?,
            custom_fields: HashMap::new(),
        };

        // Extract workflow-specific fields
        let name = row.try_get("name")?;
        let description = row.try_get("description")?;
        let trigger = row.try_get("trigger")?;
        let first_step = row.try_get("first_step")?;
        
        // Get steps as JSON value and convert it
        let steps_json: serde_json::Value = row.try_get("steps")?;
        let steps_vec: Vec<WorkflowStep> = serde_json::from_value(steps_json)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "steps".to_string(),
                source: Box::new(e),
            })?;
        
        let enabled = row.try_get("enabled")?;
        let timeout_seconds = row.try_get("timeout_seconds")?;
        let max_retries = row.try_get("max_retries")?;
        let keep_history = row.try_get("keep_history")?;

        Ok(Self {
            base,
            name,
            description,
            trigger,
            first_step,
            steps: WorkflowSteps(steps_vec),
            enabled,
            timeout_seconds,
            max_retries,
            keep_history,
        })
    }
}
