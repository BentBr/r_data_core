mod from;
mod processor;
mod to;
mod transform;

use anyhow::{bail, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

pub use from::{EntityFilter, FromDef};
pub use processor::DslProcessor;
pub use to::{EntityWriteMode, OutputMode, ToDef};
pub use transform::{ArithmeticOp, ArithmeticTransform, Operand, Transform};

/// Strict, explicit DSL step tying together from → transform → to
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DslStep {
    pub from: FromDef,
    pub to: ToDef,
    pub transform: Transform,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslProgram {
    pub steps: Vec<DslStep>,
}

impl DslProgram {
    pub fn from_config(config: &Value) -> anyhow::Result<Self> {
        let steps_val = config
            .get("steps")
            .ok_or_else(|| anyhow::anyhow!("Workflow config missing 'dsl' array"))?;
        let steps = steps_val
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("'dsl' must be an array"))?;

        let parsed: Vec<DslStep> = steps
            .iter()
            .cloned()
            .map(|v| serde_json::from_value::<DslStep>(v).context("Invalid DSL step"))
            .collect::<Result<_, _>>()?;
        Ok(DslProgram { steps: parsed })
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.steps.is_empty() {
            bail!("DSL must contain at least one step");
        }
        let safe_field = Regex::new(r"^[A-Za-z_][A-Za-z0-9_\.]*$").unwrap();
        for (idx, step) in self.steps.iter().enumerate() {
            from::validate_from(idx, &step.from, &safe_field)?;
            to::validate_to(idx, &step.to, &safe_field)?;
            transform::validate_transform(idx, &step.transform, &safe_field)?;
        }
        Ok(())
    }

    /// Apply a single-step-at-a-time process:
    /// 1) normalize input using from.mapping
    /// 2) transform (arithmetic) using operands (fields or constants)
    /// 3) map to output using to.mapping (returned result is the last produced)
    pub fn apply(&self, input: &Value) -> anyhow::Result<Value> {
        let mut last = json!({});
        for step in &self.steps {
            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            for (src, dst) in mapping.iter() {
                let v = get_nested(input, src).unwrap_or(Value::Null);
                set_nested(&mut normalized, dst, v);
            }
            // Transform
            if let Transform::Arithmetic(ar) = &step.transform {
                let left = eval_operand(&normalized, &ar.left);
                let right = eval_operand(&normalized, &ar.right);
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
                    set_nested(&mut normalized, &ar.target, Value::from(new_val));
                }
            }
            // Map to output
            let out_mapping = to::mapping_of(&step.to);
            let mut produced = json!({});
            for (src, dst) in out_mapping.iter() {
                let v = get_nested(&normalized, src).unwrap_or(Value::Null);
                set_nested(&mut produced, dst, v);
            }
            last = produced;
        }
        Ok(last)
    }
}

fn eval_operand(ctx: &Value, op: &Operand) -> Option<f64> {
    match op {
        Operand::Field { field } => get_nested(ctx, field).and_then(|v| v.as_f64()),
        Operand::Const { value } => Some(*value),
        Operand::ExternalEntityField { .. } => {
            // Future: resolve from repository; for now not supported in apply
            None
        }
    }
}

pub(crate) fn validate_mapping(
    idx: usize,
    mapping: &std::collections::HashMap<String, String>,
    safe_field: &Regex,
) -> anyhow::Result<()> {
    if mapping.is_empty() {
        bail!("DSL step {}: mapping must contain at least one field", idx);
    }
    for (k, v) in mapping {
        if !safe_field.is_match(k) || !safe_field.is_match(v) {
            bail!(
                "DSL step {}: mapping contains unsafe field names ('{}' -> '{}')",
                idx,
                k,
                v
            );
        }
    }
    Ok(())
}

fn get_nested(input: &Value, path: &str) -> Option<Value> {
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

fn set_nested(target: &mut Value, path: &str, val: Value) {
    let mut acc = val;
    for key in path.split('.').rev() {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), acc);
        acc = Value::Object(map);
    }
    merge_objects(target, &acc);
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_ok_and_apply_arithmetic_field_const() {
        let config = json!({
            "steps": [{
                "from": { "type": "csv", "uri": "http://example/csv", "mapping": { "price": "price" } },
                "transform": {
                    "type": "arithmetic",
                    "target": "price",
                    "left": { "kind": "field", "field": "price" },
                    "op": "add",
                    "right": { "kind": "const", "value": 5.0 }
                },
                "to": { "type": "json", "output": "api", "mapping": { "price": "entity.total" } }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({ "price": 10.0 });
        let out = prog.apply(&input).unwrap();
        assert_eq!(out["entity"]["total"], json!(15.0));
    }

    #[test]
    fn test_validate_fails_on_empty_mapping() {
        let config = json!({
            "steps": [{
                "from": { "type": "csv", "uri": "x", "mapping": {} },
                "transform": {
                    "type": "arithmetic",
                    "target": "x",
                    "left": { "kind": "const", "value": 1.0 },
                    "op": "add",
                    "right": { "kind": "const", "value": 2.0 }
                },
                "to": { "type": "json", "output": "api", "mapping": {} }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        let err = prog.validate().unwrap_err();
        assert!(err
            .to_string()
            .contains("mapping must contain at least one field"));
    }
}
