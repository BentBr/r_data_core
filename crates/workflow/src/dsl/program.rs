#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::from;
use super::to;
use super::transform::{ArithmeticOp, Transform};
use super::DslStep;

/// DSL program containing multiple steps
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslProgram {
    /// Steps in the program
    pub steps: Vec<DslStep>,
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
        let steps_val = config
            .get("steps")
            .ok_or_else(|| r_data_core_core::error::Error::Validation("Workflow config missing 'steps' array".to_string()))?;
        let steps = steps_val
            .as_array()
            .ok_or_else(|| r_data_core_core::error::Error::Validation("'steps' must be an array".to_string()))?;

        let parsed: Vec<DslStep> = steps
            .iter()
            .cloned()
            .map(|v| serde_json::from_value::<DslStep>(v).map_err(|e| r_data_core_core::error::Error::Validation(format!("Invalid DSL step: {e}"))))
            .collect::<r_data_core_core::error::Result<_>>()?;
        Ok(Self { steps: parsed })
    }

    /// Validate the DSL program
    ///
    /// # Errors
    /// Returns an error if validation fails
    ///
    /// # Panics
    /// Panics if the regex pattern is invalid (should never happen).
    pub fn validate(&self) -> r_data_core_core::error::Result<()> {
        if self.steps.is_empty() {
            return Err(r_data_core_core::error::Error::Validation("DSL must contain at least one step".to_string()));
        }
        let safe_field = Regex::new(r"^[A-Za-z_][A-Za-z0-9_\.]*$").unwrap();
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
    pub fn execute(&self, input: &Value) -> r_data_core_core::error::Result<Vec<(super::to::ToDef, Value)>> {
        use super::execution;
        use super::from::FromDef;

        let mut results: Vec<(super::to::ToDef, Value)> = Vec::new();
        let mut step_outputs: Vec<Value> = Vec::new(); // Store normalized data from each step

        for (step_idx, step) in self.steps.iter().enumerate() {
            // Determine source data based on FromDef type
            let source_data = match &step.from {
                FromDef::PreviousStep { .. } => {
                    // Read from previous step's normalized data
                    if step_idx == 0 {
                        return Err(r_data_core_core::error::Error::Validation("Step 0 cannot use PreviousStep source".to_string()));
                    }
                    &step_outputs[step_idx - 1]
                }
                FromDef::Trigger { .. } => {
                    // Trigger has no input data - use empty object
                    &serde_json::json!({})
                }
                FromDef::Format { .. } | FromDef::Entity { .. } => {
                    // Read from original input
                    input
                }
            };

            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            if mapping.is_empty() {
                // If mapping is empty, pass through all fields from source
                if let Some(source_obj) = source_data.as_object() {
                    for (k, v) in source_obj {
                        execution::set_nested(&mut normalized, k, v.clone());
                    }
                }
            } else {
                // Sort mapping entries to ensure deterministic execution
                let mut sorted_mapping: Vec<_> = mapping.iter().collect();
                sorted_mapping.sort_by_key(|(src, _)| *src);
                for (src, dst) in sorted_mapping {
                    let v = execution::get_nested(source_data, src).unwrap_or(Value::Null);
                    execution::set_nested(&mut normalized, dst, v);
                }
            }

            // Transform with proper error handling
            match &step.transform {
                Transform::Arithmetic(ar) => {
                    let left_result = execution::eval_operand(&normalized, &ar.left);
                    let right_result = execution::eval_operand(&normalized, &ar.right);

                    match (left_result, right_result) {
                        (Ok(left_val), Ok(right_val)) => {
                            let new_val = match ar.op {
                                ArithmeticOp::Add => left_val + right_val,
                                ArithmeticOp::Sub => left_val - right_val,
                                ArithmeticOp::Mul => left_val * right_val,
                                ArithmeticOp::Div => {
                                    #[allow(clippy::float_cmp)]
                                    // We explicitly want exact comparison for zero
                                    if right_val == 0.0 {
                                        return Err(r_data_core_core::error::Error::Validation(format!(
                                            "Step {step_idx}: Division by zero in target field '{}'",
                                            ar.target
                                        )));
                                    }
                                    left_val / right_val
                                }
                            };
                            execution::set_nested(
                                &mut normalized,
                                &ar.target,
                                Value::from(new_val),
                            );
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: Arithmetic error in target field '{}': {}",
                                ar.target,
                                e
                            )));
                        }
                    }
                }
                Transform::Concat(ct) => {
                    let left_result = execution::eval_string_operand(&normalized, &ct.left);
                    let right_result = execution::eval_string_operand(&normalized, &ct.right);

                    match (left_result, right_result) {
                        (Ok(left_str), Ok(right_str)) => {
                            let sep = ct.separator.as_deref().unwrap_or("");
                            let combined = format!("{left_str}{sep}{right_str}");
                            execution::set_nested(
                                &mut normalized,
                                &ct.target,
                                Value::from(combined),
                            );
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: Concat error in target field '{}': {}",
                                ct.target,
                                e
                            )));
                        }
                    }
                }
                Transform::None => {
                    // no-op
                }
            }

            // Map to output
            let out_mapping = to::mapping_of(&step.to);
            let produced = if out_mapping.is_empty() {
                // If no mapping for output, use normalized directly (pass through all fields)
                normalized.clone()
            } else {
                // Sort mapping entries by destination to ensure deterministic execution
                // This ensures reserved fields like 'path' are processed in a consistent order
                // Mapping structure: { destination_field: normalized_field }
                let mut produced = json!({});
                let mut sorted_mapping: Vec<_> = out_mapping.iter().collect();
                sorted_mapping.sort_by_key(|(dst, _)| *dst);
                for (dst, src) in sorted_mapping {
                    let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                    execution::set_nested(&mut produced, dst, v);
                }
                produced
            };

            // Store data for next step (for PreviousStep references)
            // For NextStep ToDef, store the mapped output; otherwise store normalized
            match &step.to {
                super::to::ToDef::NextStep { .. } => {
                    // For NextStep, store the mapped output so next step sees the mapped fields
                    step_outputs.push(produced.clone());
                }
                _ => {
                    // For Format/Entity, store normalized (next step reads via PreviousStep mapping)
                    step_outputs.push(normalized.clone());
                }
            }

            results.push((step.to.clone(), produced));
        }
        Ok(results)
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
        use super::execution;
        use super::from::FromDef;

        let mut last = json!({});
        let mut step_outputs: Vec<Value> = Vec::new(); // Store normalized data from each step

        for (step_idx, step) in self.steps.iter().enumerate() {
            // Determine source data based on FromDef type
            let source_data = match &step.from {
                FromDef::PreviousStep { .. } => {
                    // Read from previous step's normalized data
                    if step_idx == 0 {
                        return Err(r_data_core_core::error::Error::Validation("Step 0 cannot use PreviousStep source".to_string()));
                    }
                    &step_outputs[step_idx - 1]
                }
                FromDef::Trigger { .. } => {
                    // Trigger has no input data - use empty object
                    &serde_json::json!({})
                }
                FromDef::Format { .. } | FromDef::Entity { .. } => {
                    // Read from original input
                    input
                }
            };

            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            if mapping.is_empty() {
                // If mapping is empty, pass through all fields from source
                if let Some(source_obj) = source_data.as_object() {
                    for (k, v) in source_obj {
                        execution::set_nested(&mut normalized, k, v.clone());
                    }
                }
            } else {
                // Sort mapping entries to ensure deterministic execution
                let mut sorted_mapping: Vec<_> = mapping.iter().collect();
                sorted_mapping.sort_by_key(|(src, _)| *src);
                for (src, dst) in sorted_mapping {
                    let v = execution::get_nested(source_data, src).unwrap_or(Value::Null);
                    execution::set_nested(&mut normalized, dst, v);
                }
            }

            // Transform with proper error handling
            match &step.transform {
                Transform::Arithmetic(ar) => {
                    let left_result = execution::eval_operand(&normalized, &ar.left);
                    let right_result = execution::eval_operand(&normalized, &ar.right);

                    match (left_result, right_result) {
                        (Ok(left_val), Ok(right_val)) => {
                            let new_val = match ar.op {
                                ArithmeticOp::Add => left_val + right_val,
                                ArithmeticOp::Sub => left_val - right_val,
                                ArithmeticOp::Mul => left_val * right_val,
                                ArithmeticOp::Div => {
                                    #[allow(clippy::float_cmp)]
                                    // We explicitly want exact comparison for zero
                                    if right_val == 0.0 {
                                        return Err(r_data_core_core::error::Error::Validation(format!(
                                            "Step {step_idx}: Division by zero in target field '{}'",
                                            ar.target
                                        )));
                                    }
                                    left_val / right_val
                                }
                            };
                            execution::set_nested(
                                &mut normalized,
                                &ar.target,
                                Value::from(new_val),
                            );
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: Arithmetic error in target field '{}': {}",
                                ar.target,
                                e
                            )));
                        }
                    }
                }
                Transform::Concat(ct) => {
                    let left_result = execution::eval_string_operand(&normalized, &ct.left);
                    let right_result = execution::eval_string_operand(&normalized, &ct.right);

                    match (left_result, right_result) {
                        (Ok(left_str), Ok(right_str)) => {
                            let sep = ct.separator.as_deref().unwrap_or("");
                            let combined = format!("{left_str}{sep}{right_str}");
                            execution::set_nested(
                                &mut normalized,
                                &ct.target,
                                Value::from(combined),
                            );
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: Concat error in target field '{}': {}",
                                ct.target,
                                e
                            )));
                        }
                    }
                }
                Transform::None => {
                    // no-op
                }
            }

            // Map to output
            let out_mapping = to::mapping_of(&step.to);
            let produced = if out_mapping.is_empty() {
                // If no mapping for output, use normalized directly (pass through all fields)
                normalized.clone()
            } else {
                // Sort mapping entries by destination to ensure deterministic execution
                // This ensures reserved fields like 'path' are processed in a consistent order
                // Mapping structure: { destination_field: normalized_field }
                let mut produced = json!({});
                let mut sorted_out_mapping: Vec<_> = out_mapping.iter().collect();
                sorted_out_mapping.sort_by_key(|(dst, _)| *dst);
                for (dst, src) in sorted_out_mapping {
                    let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                    execution::set_nested(&mut produced, dst, v);
                }
                produced
            };

            // Store data for next step (for PreviousStep references)
            // For NextStep ToDef, store the mapped output; otherwise store normalized
            match &step.to {
                super::to::ToDef::NextStep { .. } => {
                    // For NextStep, store the mapped output so next step sees the mapped fields
                    step_outputs.push(produced.clone());
                }
                _ => {
                    // For Format/Entity, store normalized (next step reads via PreviousStep mapping)
                    step_outputs.push(normalized.clone());
                }
            }

            last = produced;
        }
        Ok(last)
    }
}
