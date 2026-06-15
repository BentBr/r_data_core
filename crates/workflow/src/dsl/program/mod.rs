#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod execute;
mod step_helpers;

#[cfg(test)]
mod tests;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::from;
use super::on_complete::OnComplete;
use super::to;
use super::transform::Transform;
use super::DslStep;

pub use execute::{apply, execute};
pub use step_helpers::{apply_build_path, finalize_step, get_next_step_input, prepare_step};

/// DSL program containing multiple steps
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslProgram {
    /// Steps in the program
    pub steps: Vec<DslStep>,
    /// Optional post-run actions executed once after all items are processed
    #[serde(default)]
    pub on_complete: Option<OnComplete>,
}

impl DslProgram {
    /// Create a DSL program from a configuration value
    ///
    /// # Arguments
    /// * `config` - JSON configuration containing a "steps" array
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub fn from_config(config: &Value) -> r_data_core_core::error::Result<Self> {
        let steps_val = config.get("steps").ok_or_else(|| {
            r_data_core_core::error::Error::Validation(
                "Workflow config missing 'steps' array".to_string(),
            )
        })?;
        let steps = steps_val.as_array().ok_or_else(|| {
            r_data_core_core::error::Error::Validation("'steps' must be an array".to_string())
        })?;

        let parsed: Vec<DslStep> = steps
            .iter()
            .cloned()
            .map(|v| {
                serde_json::from_value::<DslStep>(v).map_err(|e| {
                    r_data_core_core::error::Error::Validation(format!("Invalid DSL step: {e}"))
                })
            })
            .collect::<r_data_core_core::error::Result<_>>()?;

        let on_complete: Option<OnComplete> = config
            .get("on_complete")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(Self {
            steps: parsed,
            on_complete,
        })
    }

    /// Validate the DSL program
    ///
    /// # Errors
    /// Returns an error if validation fails
    ///
    pub fn validate(&self) -> r_data_core_core::error::Result<()> {
        if self.steps.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "DSL must contain at least one step".to_string(),
            ));
        }
        let safe_field = Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").map_err(|e| {
            r_data_core_core::error::Error::Config(format!(
                "Failed to compile field validation regex: {e}"
            ))
        })?;
        let last_step_idx = self.steps.len() - 1;
        for (idx, step) in self.steps.iter().enumerate() {
            from::validate_from(idx, &step.from, &safe_field)?;
            to::validate_to(idx, &step.to, &safe_field)?;
            super::transform::validate_transform(idx, &step.transform, &safe_field)?;
            // NextStep cannot be used in the last step
            if idx == last_step_idx {
                if let super::to::ToDef::NextStep { .. } = &step.to {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "Step {idx} (last step) cannot use NextStep ToDef - there is no next step"
                    )));
                }
            }
        }
        if let Some(ref oc) = self.on_complete {
            super::on_complete::validate_on_complete(oc, &safe_field)?;
        }
        Ok(())
    }

    /// Execute all steps and return produced outputs per step along with their target (`to`) definitions.
    /// Supports step chaining via `PreviousStep` `FromDef` type.
    ///
    /// # Arguments
    /// * `input` - Input JSON value
    ///
    /// # Errors
    /// Returns an error if execution fails
    #[allow(clippy::too_many_lines)] // Complex but cohesive function
    pub fn execute(
        &self,
        input: &Value,
    ) -> r_data_core_core::error::Result<Vec<(super::to::ToDef, Value)>> {
        execute(self, input)
    }

    /// Apply a single-step-at-a-time process and return the last produced value.
    /// Supports step chaining via `PreviousStep` `FromDef` type.
    /// 1) normalize input using from.mapping
    /// 2) transform (arithmetic/concat) using operands
    /// 3) map to output using to.mapping (returned result is the last produced)
    ///
    /// # Arguments
    /// * `input` - Input JSON value
    ///
    /// # Errors
    /// Returns an error if execution fails
    #[allow(clippy::too_many_lines)]
    pub fn apply(&self, input: &Value) -> r_data_core_core::error::Result<Value> {
        apply(self, input)
    }

    /// Execute a single step and return normalized data before output mapping.
    /// This allows async transforms to be injected into normalized before finalizing.
    ///
    /// # Arguments
    /// * `step_idx` - Index of the step to execute
    /// * `original_input` - Original workflow input (used for Format/Entity sources)
    /// * `previous_step_output` - Output from the previous step (used for `PreviousStep` source)
    ///
    /// # Returns
    /// `(normalized, transform)` - The normalized data and the step's transform type.
    /// For async transforms (`ResolveEntityPath`, `GetOrCreateEntity`), the caller should
    /// execute the transform and inject results into `normalized` before calling `finalize_step`.
    ///
    /// # Errors
    /// Returns an error if step execution fails
    pub fn prepare_step(
        &self,
        step_idx: usize,
        original_input: &Value,
        previous_step_output: Option<&Value>,
    ) -> r_data_core_core::error::Result<(Value, &Transform)> {
        prepare_step(self, step_idx, original_input, previous_step_output)
    }

    /// Apply `BuildPath` transform to normalized data.
    /// This is separated because it may depend on async transform results.
    ///
    /// # Arguments
    /// * `step_idx` - Step index (for error messages)
    /// * `transform` - The transform to apply
    /// * `normalized` - Mutable normalized data to update
    ///
    /// # Errors
    /// Returns an error if the transform fails
    pub fn apply_build_path(
        step_idx: usize,
        transform: &Transform,
        normalized: &mut Value,
    ) -> r_data_core_core::error::Result<()> {
        apply_build_path(step_idx, transform, normalized)
    }

    /// Finalize a step by applying output mapping.
    ///
    /// # Arguments
    /// * `step_idx` - Index of the step
    /// * `normalized` - Normalized data (with async transform results injected if needed)
    ///
    /// # Returns
    /// `(ToDef, produced)` - The step's target definition and produced output
    ///
    /// # Errors
    /// Returns an error if step index is out of bounds
    pub fn finalize_step(
        &self,
        step_idx: usize,
        normalized: &Value,
    ) -> r_data_core_core::error::Result<(super::to::ToDef, Value)> {
        finalize_step(self, step_idx, normalized)
    }

    /// Determine what should be passed to the next step based on `ToDef` type.
    ///
    /// # Arguments
    /// * `step_idx` - Index of the current step
    /// * `normalized` - Normalized data from current step
    /// * `produced` - Produced output from current step
    ///
    /// # Returns
    /// The value to pass to the next step
    ///
    /// # Errors
    /// Returns an error if step index is out of bounds
    pub fn get_next_step_input(
        &self,
        step_idx: usize,
        normalized: &Value,
        produced: &Value,
    ) -> r_data_core_core::error::Result<Value> {
        get_next_step_input(self, step_idx, normalized, produced)
    }
}
