#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::{bail, Context};
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
    pub fn from_config(config: &Value) -> anyhow::Result<Self> {
        let steps_val = config
            .get("steps")
            .ok_or_else(|| anyhow::anyhow!("Workflow config missing 'steps' array"))?;
        let steps = steps_val
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("'steps' must be an array"))?;

        let parsed: Vec<DslStep> = steps
            .iter()
            .cloned()
            .map(|v| serde_json::from_value::<DslStep>(v).context("Invalid DSL step"))
            .collect::<Result<_, _>>()?;
        Ok(DslProgram { steps: parsed })
    }

    /// Validate the DSL program
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.steps.is_empty() {
            bail!("DSL must contain at least one step");
        }
        let safe_field = Regex::new(r"^[A-Za-z_][A-Za-z0-9_\.]*$").unwrap();
        for (idx, step) in self.steps.iter().enumerate() {
            from::validate_from(idx, &step.from, &safe_field)?;
            to::validate_to(idx, &step.to, &safe_field)?;
            super::transform::validate_transform(idx, &step.transform, &safe_field)?;
        }
        Ok(())
    }

    /// Execute all steps and return produced outputs per step along with their target (`to`) definitions.
    /// For `to.entity` with empty mapping, we return the normalized object for that step.
    ///
    /// # Arguments
    /// * `input` - Input JSON value
    ///
    /// # Errors
    /// Returns an error if execution fails
    pub fn execute(&self, input: &Value) -> anyhow::Result<Vec<(super::to::ToDef, Value)>> {
        use super::execution;
        let mut results: Vec<(super::to::ToDef, Value)> = Vec::new();
        for step in &self.steps {
            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            if mapping.is_empty() {
                // If mapping is empty, pass through all fields from input
                if let Some(input_obj) = input.as_object() {
                    for (k, v) in input_obj {
                        execution::set_nested(&mut normalized, k, v.clone());
                    }
                }
            } else {
                // Sort mapping entries to ensure deterministic execution
                let mut sorted_mapping: Vec<_> = mapping.iter().collect();
                sorted_mapping.sort_by_key(|(src, _)| *src);
                for (src, dst) in sorted_mapping {
                    let v = execution::get_nested(input, src).unwrap_or(Value::Null);
                    execution::set_nested(&mut normalized, dst, v);
                }
            }
            // Transform
            match &step.transform {
                Transform::Arithmetic(ar) => {
                    let left = execution::eval_operand(&normalized, &ar.left);
                    let right = execution::eval_operand(&normalized, &ar.right);
                    if let (Some(ln), Some(rn)) = (left, right) {
                        let new_val = match ar.op {
                            ArithmeticOp::Add => ln + rn,
                            ArithmeticOp::Sub => ln - rn,
                            ArithmeticOp::Mul => ln * rn,
                            ArithmeticOp::Div => {
                                if rn == 0.0 {
                                    ln
                                } else {
                                    ln / rn
                                }
                            }
                        };
                        execution::set_nested(&mut normalized, &ar.target, Value::from(new_val));
                    }
                }
                Transform::Concat(ct) => {
                    let left =
                        execution::eval_string_operand(&normalized, &ct.left).unwrap_or_default();
                    let right =
                        execution::eval_string_operand(&normalized, &ct.right).unwrap_or_default();
                    let sep = ct.separator.clone().unwrap_or_default();
                    let combined = if sep.is_empty() {
                        format!("{}{}", left, right)
                    } else {
                        format!("{}{}{}", left, sep, right)
                    };
                    execution::set_nested(&mut normalized, &ct.target, Value::from(combined));
                }
                Transform::None => {
                    // no-op
                }
            }
            // Map to output
            let mut produced = json!({});
            match &step.to {
                super::to::ToDef::Entity { mapping, .. } if mapping.is_empty() => {
                    // If no mapping for entity, use normalized directly
                    produced = normalized.clone();
                }
                _ => {
                    let out_mapping = to::mapping_of(&step.to);
                    // Sort mapping entries by destination to ensure deterministic execution
                    // This ensures reserved fields like 'path' are processed in a consistent order
                    // Mapping structure: { destination_field: normalized_field }
                    let mut sorted_mapping: Vec<_> = out_mapping.iter().collect();
                    sorted_mapping.sort_by_key(|(dst, _)| *dst);
                    for (dst, src) in sorted_mapping {
                        let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                        execution::set_nested(&mut produced, dst, v);
                    }
                }
            }
            results.push((step.to.clone(), produced));
        }
        Ok(results)
    }

    /// Apply a single-step-at-a-time process:
    /// 1) normalize input using from.mapping
    /// 2) transform (arithmetic) using operands (fields or constants)
    /// 3) map to output using to.mapping (returned result is the last produced)
    ///
    /// # Arguments
    /// * `input` - Input JSON value
    ///
    /// # Errors
    /// Returns an error if execution fails
    pub fn apply(&self, input: &Value) -> anyhow::Result<Value> {
        use super::execution;
        let mut last = json!({});
        for step in &self.steps {
            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            // Sort mapping entries to ensure deterministic execution
            let mut sorted_mapping: Vec<_> = mapping.iter().collect();
            sorted_mapping.sort_by_key(|(src, _)| *src);
            for (src, dst) in sorted_mapping {
                let v = execution::get_nested(input, src).unwrap_or(Value::Null);
                execution::set_nested(&mut normalized, dst, v);
            }
            // Transform
            match &step.transform {
                Transform::Arithmetic(ar) => {
                    let left = execution::eval_operand(&normalized, &ar.left);
                    let right = execution::eval_operand(&normalized, &ar.right);
                    if let (Some(ln), Some(rn)) = (left, right) {
                        let new_val = match ar.op {
                            ArithmeticOp::Add => ln + rn,
                            ArithmeticOp::Sub => ln - rn,
                            ArithmeticOp::Mul => ln * rn,
                            ArithmeticOp::Div => {
                                if rn == 0.0 {
                                    ln
                                } else {
                                    ln / rn
                                }
                            }
                        };
                        execution::set_nested(&mut normalized, &ar.target, Value::from(new_val));
                    }
                }
                Transform::Concat(ct) => {
                    let left =
                        execution::eval_string_operand(&normalized, &ct.left).unwrap_or_default();
                    let right =
                        execution::eval_string_operand(&normalized, &ct.right).unwrap_or_default();
                    let sep = ct.separator.clone().unwrap_or_default();
                    let combined = if sep.is_empty() {
                        format!("{}{}", left, right)
                    } else {
                        format!("{}{}{}", left, sep, right)
                    };
                    execution::set_nested(&mut normalized, &ct.target, Value::from(combined));
                }
                Transform::None => {
                    // no-op
                }
            }
            // Map to output
            let out_mapping = to::mapping_of(&step.to);
            let mut produced = json!({});
            // Sort mapping entries by destination to ensure deterministic execution
            // This ensures reserved fields like 'path' are processed in a consistent order
            // Mapping structure: { destination_field: normalized_field }
            let mut sorted_out_mapping: Vec<_> = out_mapping.iter().collect();
            sorted_out_mapping.sort_by_key(|(dst, _)| *dst);
            for (dst, src) in sorted_out_mapping {
                let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                execution::set_nested(&mut produced, dst, v);
            }
            last = produced;
        }
        Ok(last)
    }
}
