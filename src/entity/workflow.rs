use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sqlx::{FromRow, postgres::PgRow, Row};
use std::collections::HashMap;
use serde_json;
use std::default::Default;
use uuid;
use utoipa::ToSchema;

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
    /// Step ID
    pub id: Uuid,
    
    /// Step name
    pub name: String,
    
    /// Step description
    pub description: Option<String>,
    
    /// Action to perform
    pub action: ActionType,
    
    /// Step configuration as JSON
    pub config: serde_json::Value,
    
    /// Next step IDs (empty for terminal steps)
    pub next_steps: Vec<Uuid>,
    
    /// Condition for branching (optional)
    pub condition: Option<String>,
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
    pub steps: Vec<WorkflowStep>,
    
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
            steps: Vec::new(),
            enabled: false,
            timeout_seconds: 300,
            max_retries: 3,
            keep_history: true,
        }
    }
    
    /// Add a step to the workflow
    pub fn add_step(&mut self, step: WorkflowStep) -> Result<()> {
        // Check if the step ID already exists
        if self.steps.iter().any(|s| s.id == step.id) {
            return Err(Error::Workflow(format!("Step with ID {} already exists", step.id)));
        }
        
        // Validate transitions against existing steps
        for transition in &step.next_steps {
            if !self.steps.iter().any(|s| s.id == *transition) && 
               *transition != step.id {
                return Err(Error::Workflow(format!(
                    "Transition target step ID {} does not exist", 
                    *transition
                )));
            }
        }
        
        self.steps.push(step);
        Ok(())
    }
    
    /// Get a step by ID
    pub fn get_step(&self, id: Uuid) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == id)
    }
    
    /// Remove a step from the workflow
    pub fn remove_step(&mut self, id: Uuid) -> Result<()> {
        // Check if any other steps have transitions to this step
        let step_idx = self.steps.iter().position(|s| s.id == id);
        
        if let Some(idx) = step_idx {
            // Check if any other steps reference this step in transitions
            for step in &self.steps {
                if step.id != id && step.next_steps.iter().any(|t| t == &id) {
                    return Err(Error::Workflow(
                        format!("Cannot remove step {} because step {} has a transition to it", 
                        id, step.id)
                    ));
                }
            }
            
            self.steps.remove(idx);
            Ok(())
        } else {
            Err(Error::Workflow(format!("Step with ID {} not found", id)))
        }
    }
    
    /// Validate the workflow
    pub fn validate(&self) -> Result<()> {
        // Ensure we have at least one step
        if self.steps.is_empty() {
            return Err(Error::Workflow("Workflow must have at least one step".to_string()));
        }
        
        // Ensure we have a valid initial step
        if let Some(first_id) = self.first_step {
            if !self.steps.iter().any(|s| s.id == first_id) {
                return Err(Error::Workflow(format!(
                    "Initial step ID {} does not exist in steps", 
                    first_id
                )));
            }
        }
        
        // Validate all transitions point to valid step IDs
        for step in &self.steps {
            for &next_id in &step.next_steps {
                if !self.steps.iter().any(|s| s.id == next_id) {
                    return Err(Error::Workflow(format!(
                        "Transition in step {} points to non-existent step {}", 
                        step.id, next_id
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
        let id = row.try_get::<i64, _>("id").ok();
        let uuid_str = row.try_get::<String, _>("uuid")?;
        let uuid = uuid::Uuid::parse_str(&uuid_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "uuid".to_string(),
            source: Box::new(e)
        })?;
        
        let base = AbstractRDataEntity {
            id,
            uuid,
            path: row.try_get("path")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            created_by: row.try_get("created_by").ok(),
            updated_by: row.try_get("updated_by").ok(),
            published: row.try_get("published").unwrap_or(false),
            version: row.try_get("version").unwrap_or(1),
            custom_fields: HashMap::new(), // We can populate this if needed
        };
        
        // Extract main fields
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description").ok();
        let trigger: TriggerType = serde_json::from_value(row.try_get("trigger")?).unwrap_or_default();
        let enabled: bool = row.try_get("enabled").unwrap_or(false);
        let timeout_seconds: i32 = row.try_get("timeout_seconds").unwrap_or(300);
        let max_retries: i32 = row.try_get("max_retries").unwrap_or(0);
        let keep_history: bool = row.try_get("keep_history").unwrap_or(true);
        
        // Extract JSON fields for first_step
        let first_step_str: Option<String> = match row.try_get::<serde_json::Value, _>("first_step") {
            Ok(json) => serde_json::from_value(json).ok(),
            Err(_) => None,
        };
        
        // Convert to UUID if present
        let first_step = first_step_str.and_then(|s| uuid::Uuid::parse_str(&s).ok());
        
        // Extract steps as a HashMap and then convert to Vec
        let steps_map: HashMap<String, WorkflowStep> = match row.try_get::<serde_json::Value, _>("steps") {
            Ok(json) => serde_json::from_value(json).unwrap_or_default(),
            Err(_) => HashMap::new(),
        };
        
        // Convert HashMap to Vec
        let steps = steps_map.into_values().collect();
        
        Ok(WorkflowEntity {
            base,
            name,
            description,
            trigger,
            first_step,
            steps,
            enabled,
            timeout_seconds,
            max_retries,
            keep_history,
        })
    }
} 