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
        Ok(Self { steps: parsed })
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
                        return Err(r_data_core_core::error::Error::Validation(
                            "Step 0 cannot use PreviousStep source".to_string(),
                        ));
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
                                ar.target, e
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
                                ct.target, e
                            )));
                        }
                    }
                }
                Transform::BuildPath(bp) => {
                    use super::path_resolution::build_path_from_fields;
                    match build_path_from_fields::<std::collections::hash_map::RandomState>(
                        &bp.template,
                        &normalized,
                        bp.separator.as_deref(),
                        bp.field_transforms.as_ref(),
                    ) {
                        Ok(path) => {
                            execution::set_nested(&mut normalized, &bp.target, Value::String(path));
                        }
                        Err(e) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: BuildPath error in target field '{}': {}",
                                bp.target, e
                            )));
                        }
                    }
                }
                Transform::ResolveEntityPath(_)
                | Transform::GetOrCreateEntity(_)
                | Transform::Authenticate(_)
                | Transform::None => {
                    // ResolveEntityPath, GetOrCreateEntity, and Authenticate require async database
                    // access and are handled in the services layer during workflow execution.
                    // They are validated here but execution happens in services.
                    // Transform::None is a no-op.
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
                        return Err(r_data_core_core::error::Error::Validation(
                            "Step 0 cannot use PreviousStep source".to_string(),
                        ));
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
                                ar.target, e
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
                                ct.target, e
                            )));
                        }
                    }
                }
                Transform::BuildPath(bp) => {
                    use super::path_resolution::build_path_from_fields;
                    match build_path_from_fields::<std::collections::hash_map::RandomState>(
                        &bp.template,
                        &normalized,
                        bp.separator.as_deref(),
                        bp.field_transforms.as_ref(),
                    ) {
                        Ok(path) => {
                            execution::set_nested(&mut normalized, &bp.target, Value::String(path));
                        }
                        Err(e) => {
                            return Err(r_data_core_core::error::Error::Validation(format!(
                                "Step {step_idx}: BuildPath error in target field '{}': {}",
                                bp.target, e
                            )));
                        }
                    }
                }
                Transform::ResolveEntityPath(_)
                | Transform::GetOrCreateEntity(_)
                | Transform::Authenticate(_)
                | Transform::None => {
                    // ResolveEntityPath, GetOrCreateEntity, and Authenticate require async database
                    // access and are handled in the services layer during workflow execution.
                    // They are validated here but execution happens in services.
                    // Transform::None is a no-op.
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
        use super::execution;
        use super::from::FromDef;

        let step = self.steps.get(step_idx).ok_or_else(|| {
            r_data_core_core::error::Error::Validation(format!(
                "Step index {step_idx} out of bounds"
            ))
        })?;

        // Determine source data based on FromDef type
        let empty_obj = json!({});
        let source_data = match &step.from {
            FromDef::PreviousStep { .. } => {
                if step_idx == 0 {
                    return Err(r_data_core_core::error::Error::Validation(
                        "Step 0 cannot use PreviousStep source".to_string(),
                    ));
                }
                previous_step_output.ok_or_else(|| {
                    r_data_core_core::error::Error::Validation(
                        "PreviousStep source requires previous step output".to_string(),
                    )
                })?
            }
            FromDef::Trigger { .. } => &empty_obj,
            FromDef::Format { .. } | FromDef::Entity { .. } => original_input,
        };

        // Normalize
        let mut normalized = json!({});
        let mapping = from::mapping_of(&step.from);
        if mapping.is_empty() {
            if let Some(source_obj) = source_data.as_object() {
                for (k, v) in source_obj {
                    execution::set_nested(&mut normalized, k, v.clone());
                }
            }
        } else {
            let mut sorted_mapping: Vec<_> = mapping.iter().collect();
            sorted_mapping.sort_by_key(|(src, _)| *src);
            for (src, dst) in sorted_mapping {
                let v = execution::get_nested(source_data, src).unwrap_or(Value::Null);
                execution::set_nested(&mut normalized, dst, v);
            }
        }

        // Apply sync transforms only
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
                                if right_val == 0.0 {
                                    return Err(r_data_core_core::error::Error::Validation(
                                        format!(
                                            "Step {step_idx}: Division by zero in target field '{}'",
                                            ar.target
                                        ),
                                    ));
                                }
                                left_val / right_val
                            }
                        };
                        execution::set_nested(&mut normalized, &ar.target, Value::from(new_val));
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        return Err(r_data_core_core::error::Error::Validation(format!(
                            "Step {step_idx}: Arithmetic error in target field '{}': {}",
                            ar.target, e
                        )));
                    }
                }
            }
            Transform::Concat(ct) => {
                let left_result = execution::eval_string_operand(&normalized, &ct.left);
                let right_result = execution::eval_string_operand(&normalized, &ct.right);

                match (left_result, right_result) {
                    (Ok(left_str), Ok(right_str)) => {
                        let separator = ct.separator.as_deref().unwrap_or("");
                        let combined = format!("{left_str}{separator}{right_str}");
                        execution::set_nested(&mut normalized, &ct.target, Value::from(combined));
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        return Err(r_data_core_core::error::Error::Validation(format!(
                            "Step {step_idx}: Concat error in target field '{}': {}",
                            ct.target, e
                        )));
                    }
                }
            }
            // Async transforms are NOT executed here - caller must handle them
            Transform::ResolveEntityPath(_)
            | Transform::GetOrCreateEntity(_)
            | Transform::Authenticate(_)
            | Transform::BuildPath(_)
            | Transform::None => {}
        }

        Ok((normalized, &step.transform))
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
        use super::execution;

        if let Transform::BuildPath(bp) = transform {
            use super::path_resolution::build_path_from_fields;
            match build_path_from_fields::<std::collections::hash_map::RandomState>(
                &bp.template,
                normalized,
                bp.separator.as_deref(),
                bp.field_transforms.as_ref(),
            ) {
                Ok(path) => {
                    execution::set_nested(normalized, &bp.target, Value::String(path));
                }
                Err(e) => {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "Step {step_idx}: BuildPath error in target field '{}': {}",
                        bp.target, e
                    )));
                }
            }
        }
        Ok(())
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
        use super::execution;

        let step = self.steps.get(step_idx).ok_or_else(|| {
            r_data_core_core::error::Error::Validation(format!(
                "Step index {step_idx} out of bounds"
            ))
        })?;

        let out_mapping = to::mapping_of(&step.to);
        let produced = if out_mapping.is_empty() {
            normalized.clone()
        } else {
            let mut produced = json!({});
            let mut sorted_mapping: Vec<_> = out_mapping.iter().collect();
            sorted_mapping.sort_by_key(|(dst, _)| *dst);
            for (dst, src) in sorted_mapping {
                // Check if the source is a literal value (e.g., @literal:true)
                // Otherwise, read from the normalized input data
                let v = execution::parse_literal_value(src)
                    .or_else(|| execution::get_nested(normalized, src))
                    .unwrap_or(Value::Null);
                execution::set_nested(&mut produced, dst, v);
            }
            produced
        };

        Ok((step.to.clone(), produced))
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
        let step = self.steps.get(step_idx).ok_or_else(|| {
            r_data_core_core::error::Error::Validation(format!(
                "Step index {step_idx} out of bounds"
            ))
        })?;

        Ok(match &step.to {
            super::to::ToDef::NextStep { .. } => produced.clone(),
            _ => normalized.clone(),
        })
    }
}
