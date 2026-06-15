#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::{json, Value};

use crate::dsl::execution;
use crate::dsl::from::{self, FromDef};
use crate::dsl::to;
use crate::dsl::transform::{ArithmeticOp, Transform};

use super::DslProgram;

/// Execute all steps and return produced outputs per step along with their target (`to`) definitions.
/// Supports step chaining via `PreviousStep` `FromDef` type.
///
/// # Errors
/// Returns an error if execution fails
#[allow(clippy::too_many_lines)] // Complex but cohesive function
pub fn execute(
    program: &DslProgram,
    input: &Value,
) -> r_data_core_core::error::Result<Vec<(crate::dsl::to::ToDef, Value)>> {
    let mut results: Vec<(crate::dsl::to::ToDef, Value)> = Vec::new();
    let mut step_outputs: Vec<Value> = Vec::new();

    for (step_idx, step) in program.steps.iter().enumerate() {
        let source_data = match &step.from {
            FromDef::PreviousStep { .. } => {
                if step_idx == 0 {
                    return Err(r_data_core_core::error::Error::Validation(
                        "Step 0 cannot use PreviousStep source".to_string(),
                    ));
                }
                &step_outputs[step_idx - 1]
            }
            FromDef::Trigger { .. } => &serde_json::json!({}),
            FromDef::Format { .. } | FromDef::Entity { .. } => input,
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

        apply_sync_transform(step_idx, &step.transform, &mut normalized, false)?;

        let out_mapping = to::mapping_of(&step.to);
        let produced = if out_mapping.is_empty() {
            normalized.clone()
        } else {
            let mut produced = json!({});
            let mut sorted_mapping: Vec<_> = out_mapping.iter().collect();
            sorted_mapping.sort_by_key(|(dst, _)| *dst);
            for (dst, src) in sorted_mapping {
                let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                execution::set_nested(&mut produced, dst, v);
            }
            produced
        };

        match &step.to {
            crate::dsl::to::ToDef::NextStep { .. } => {
                step_outputs.push(produced.clone());
            }
            _ => {
                step_outputs.push(normalized.clone());
            }
        }

        results.push((step.to.clone(), produced));
    }
    Ok(results)
}

/// Apply a single-step-at-a-time process and return the last produced value.
/// Supports step chaining via `PreviousStep` `FromDef` type.
///
/// # Errors
/// Returns an error if execution fails
#[allow(clippy::too_many_lines)]
pub fn apply(program: &DslProgram, input: &Value) -> r_data_core_core::error::Result<Value> {
    let mut last = json!({});
    let mut step_outputs: Vec<Value> = Vec::new();

    for (step_idx, step) in program.steps.iter().enumerate() {
        let source_data = match &step.from {
            FromDef::PreviousStep { .. } => {
                if step_idx == 0 {
                    return Err(r_data_core_core::error::Error::Validation(
                        "Step 0 cannot use PreviousStep source".to_string(),
                    ));
                }
                &step_outputs[step_idx - 1]
            }
            FromDef::Trigger { .. } => &serde_json::json!({}),
            FromDef::Format { .. } | FromDef::Entity { .. } => input,
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

        apply_sync_transform(step_idx, &step.transform, &mut normalized, false)?;

        let out_mapping = to::mapping_of(&step.to);
        let produced = if out_mapping.is_empty() {
            normalized.clone()
        } else {
            let mut produced = json!({});
            let mut sorted_out_mapping: Vec<_> = out_mapping.iter().collect();
            sorted_out_mapping.sort_by_key(|(dst, _)| *dst);
            for (dst, src) in sorted_out_mapping {
                let v = execution::get_nested(&normalized, src).unwrap_or(Value::Null);
                execution::set_nested(&mut produced, dst, v);
            }
            produced
        };

        match &step.to {
            crate::dsl::to::ToDef::NextStep { .. } => {
                step_outputs.push(produced.clone());
            }
            _ => {
                step_outputs.push(normalized.clone());
            }
        }

        last = produced;
    }
    Ok(last)
}

/// Apply synchronous transforms (`Arithmetic`, `Concat`, `BuildPath`) to normalized data.
///
/// `Arithmetic` and `Concat` are always applied. `BuildPath` is applied inline
/// during full execution, but deferred when `defer_build_path` is set:
/// step-by-step execution applies it separately via `apply_build_path` because
/// it can depend on the results of async transforms. Async transforms
/// (`ResolveEntityPath`, `GetOrCreateEntity`, `Authenticate`, `SendEmail`) are
/// always skipped here (handled in the services layer).
pub(super) fn apply_sync_transform(
    step_idx: usize,
    transform: &Transform,
    normalized: &mut Value,
    defer_build_path: bool,
) -> r_data_core_core::error::Result<()> {
    match transform {
        Transform::Arithmetic(ar) => {
            let left_result = execution::eval_operand(normalized, &ar.left);
            let right_result = execution::eval_operand(normalized, &ar.right);

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
                    execution::set_nested(normalized, &ar.target, Value::from(new_val));
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
            let left_result = execution::eval_string_operand(normalized, &ct.left);
            let right_result = execution::eval_string_operand(normalized, &ct.right);

            match (left_result, right_result) {
                (Ok(left_str), Ok(right_str)) => {
                    let sep = ct.separator.as_deref().unwrap_or("");
                    let combined = format!("{left_str}{sep}{right_str}");
                    execution::set_nested(normalized, &ct.target, Value::from(combined));
                }
                (Err(e), _) | (_, Err(e)) => {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "Step {step_idx}: Concat error in target field '{}': {}",
                        ct.target, e
                    )));
                }
            }
        }
        Transform::BuildPath(bp) if !defer_build_path => {
            use crate::dsl::path_resolution::build_path_from_fields;
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
        // Deferred BuildPath (step-by-step mode, applied later via
        // `apply_build_path`) and async transforms (`ResolveEntityPath`/
        // `GetOrCreateEntity`/`Authenticate`/`SendEmail`, handled in the services
        // layer) are skipped here. `Transform::None` is a no-op.
        Transform::BuildPath(_)
        | Transform::ResolveEntityPath(_)
        | Transform::GetOrCreateEntity(_)
        | Transform::Authenticate(_)
        | Transform::SendEmail(_)
        | Transform::None => {}
    }
    Ok(())
}
