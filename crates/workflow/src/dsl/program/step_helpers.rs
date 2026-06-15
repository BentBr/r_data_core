#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::{json, Value};

use crate::dsl::execution;
use crate::dsl::from;
use crate::dsl::from::FromDef;
use crate::dsl::path_resolution::build_path_from_fields;
use crate::dsl::to;
use crate::dsl::transform::Transform;

use super::execute::apply_sync_transform;
use super::DslProgram;

/// Execute a single step and return normalized data before output mapping.
///
/// # Errors
/// Returns an error if step execution fails
pub fn prepare_step<'a>(
    program: &'a DslProgram,
    step_idx: usize,
    original_input: &Value,
    previous_step_output: Option<&Value>,
) -> r_data_core_core::error::Result<(Value, &'a Transform)> {
    let step = program.steps.get(step_idx).ok_or_else(|| {
        r_data_core_core::error::Error::Validation(format!("Step index {step_idx} out of bounds"))
    })?;

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

    // Apply sync transforms only (Arithmetic, Concat); BuildPath is deferred
    // (it may depend on async results) and applied later via apply_build_path.
    apply_sync_transform(step_idx, &step.transform, &mut normalized, true)?;

    Ok((normalized, &step.transform))
}

/// Apply `BuildPath` transform to normalized data.
/// This is separated because it may depend on async transform results.
///
/// # Errors
/// Returns an error if the transform fails
pub fn apply_build_path(
    step_idx: usize,
    transform: &Transform,
    normalized: &mut Value,
) -> r_data_core_core::error::Result<()> {
    if let Transform::BuildPath(bp) = transform {
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
/// # Errors
/// Returns an error if step index is out of bounds
pub fn finalize_step(
    program: &DslProgram,
    step_idx: usize,
    normalized: &Value,
) -> r_data_core_core::error::Result<(crate::dsl::to::ToDef, Value)> {
    let step = program.steps.get(step_idx).ok_or_else(|| {
        r_data_core_core::error::Error::Validation(format!("Step index {step_idx} out of bounds"))
    })?;

    let out_mapping = to::mapping_of(&step.to);
    let produced = if out_mapping.is_empty() {
        normalized.clone()
    } else {
        let mut produced = json!({});
        let mut sorted_mapping: Vec<_> = out_mapping.iter().collect();
        sorted_mapping.sort_by_key(|(dst, _)| *dst);
        for (dst, src) in sorted_mapping {
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
/// # Errors
/// Returns an error if step index is out of bounds
pub fn get_next_step_input(
    program: &DslProgram,
    step_idx: usize,
    normalized: &Value,
    produced: &Value,
) -> r_data_core_core::error::Result<Value> {
    let step = program.steps.get(step_idx).ok_or_else(|| {
        r_data_core_core::error::Error::Validation(format!("Step index {step_idx} out of bounds"))
    })?;

    Ok(match &step.to {
        crate::dsl::to::ToDef::NextStep { .. } => produced.clone(),
        _ => normalized.clone(),
    })
}
