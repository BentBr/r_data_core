#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use serde_json::Value;

use super::transform::{Operand, StringOperand};

/// Evaluate a numeric operand
///
/// # Arguments
/// * `ctx` - Context JSON value
/// * `op` - Operand to evaluate
///
/// # Returns
/// Optional f64 value if evaluation succeeds
#[must_use]
pub fn eval_operand(ctx: &Value, op: &Operand) -> Option<f64> {
    match op {
        Operand::Field { field } => get_nested(ctx, field).and_then(|v| v.as_f64()),
        Operand::Const { value } => Some(*value),
        Operand::ExternalEntityField { .. } => {
            // Future: resolve from repository; for now not supported in apply
            None
        }
    }
}

/// Evaluate a string operand
///
/// # Arguments
/// * `ctx` - Context JSON value
/// * `op` - String operand to evaluate
///
/// # Returns
/// Optional String value if evaluation succeeds
#[must_use]
pub fn eval_string_operand(ctx: &Value, op: &StringOperand) -> Option<String> {
    match op {
        StringOperand::Field { field } => {
            get_nested(ctx, field).and_then(|v| v.as_str().map(|s| s.to_string()))
        }
        StringOperand::ConstString { value } => Some(value.clone()),
    }
}

/// Get a nested value from a JSON object using dot notation
///
/// # Arguments
/// * `input` - Input JSON value
/// * `path` - Dot-separated path (e.g., "user.name")
///
/// # Returns
/// Optional Value if the path exists
#[must_use]
pub fn get_nested(input: &Value, path: &str) -> Option<Value> {
    let mut current = input;
    for key in path.split('.') {
        match current {
            Value::Object(map) => {
                if let Some(v) = map.get(key) {
                    current = v;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

/// Set a nested value in a JSON object using dot notation
///
/// # Arguments
/// * `target` - Target JSON value (will be modified)
/// * `path` - Dot-separated path (e.g., "user.name")
/// * `val` - Value to set
pub fn set_nested(target: &mut Value, path: &str, val: Value) {
    let mut acc = val;
    for key in path.split('.').rev() {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), acc);
        acc = Value::Object(map);
    }
    merge_objects(target, &acc);
}

/// Merge two JSON objects, with the addition taking precedence
///
/// # Arguments
/// * `target` - Target JSON value (will be modified)
/// * `addition` - JSON value to merge into target
fn merge_objects(target: &mut Value, addition: &Value) {
    match (target, addition) {
        (Value::Object(tobj), Value::Object(aobj)) => {
            for (k, v) in aobj {
                if let Some(existing) = tobj.get_mut(k) {
                    merge_objects(existing, v);
                } else {
                    tobj.insert(k.clone(), v.clone());
                }
            }
        }
        (t, v) => {
            *t = v.clone();
        }
    }
}

