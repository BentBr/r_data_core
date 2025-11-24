mod csv;
mod from;
mod processor;
mod to;
mod transform;

use anyhow::{bail, Context};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

// pub use csv::CsvOptions; // TODO: Re-enable when used
pub use from::{EntityFilter, FormatConfig, FromDef, SourceConfig};
// pub use processor::DslProcessor; // TODO: Re-enable when used
pub use to::{EntityWriteMode, OutputMode, ToDef};
pub use transform::{
    ArithmeticOp, ArithmeticTransform, ConcatTransform, Operand, StringOperand, Transform,
};

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

    /// Execute all steps and return produced outputs per step along with their target (`to`) definitions.
    /// For `to.entity` with empty mapping, we return the normalized object for that step.
    pub fn execute(&self, input: &Value) -> anyhow::Result<Vec<(ToDef, Value)>> {
        let mut results: Vec<(ToDef, Value)> = Vec::new();
        for step in &self.steps {
            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            if mapping.is_empty() {
                // If mapping is empty, pass through all fields from input
                if let Some(input_obj) = input.as_object() {
                    for (k, v) in input_obj {
                        set_nested(&mut normalized, k, v.clone());
                    }
                }
            } else {
                // Sort mapping entries to ensure deterministic execution
                let mut sorted_mapping: Vec<_> = mapping.iter().collect();
                sorted_mapping.sort_by_key(|(src, _)| *src);
                for (src, dst) in sorted_mapping {
                    let v = get_nested(input, src).unwrap_or(Value::Null);
                    set_nested(&mut normalized, dst, v);
                }
            }
            // Transform
            match &step.transform {
                Transform::Arithmetic(ar) => {
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
                Transform::Concat(ct) => {
                    let left = eval_string_operand(&normalized, &ct.left).unwrap_or_default();
                    let right = eval_string_operand(&normalized, &ct.right).unwrap_or_default();
                    let sep = ct.separator.clone().unwrap_or_default();
                    let combined = if sep.is_empty() {
                        format!("{}{}", left, right)
                    } else {
                        format!("{}{}{}", left, sep, right)
                    };
                    set_nested(&mut normalized, &ct.target, Value::from(combined));
                }
                Transform::None => {
                    // no-op
                }
            }
            // Map to output
            let mut produced = json!({});
            match &step.to {
                ToDef::Entity { mapping, .. } if mapping.is_empty() => {
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
                        let v = get_nested(&normalized, src).unwrap_or(Value::Null);
                        set_nested(&mut produced, dst, v);
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
    pub fn apply(&self, input: &Value) -> anyhow::Result<Value> {
        let mut last = json!({});
        for step in &self.steps {
            // Normalize
            let mut normalized = json!({});
            let mapping = from::mapping_of(&step.from);
            // Sort mapping entries to ensure deterministic execution
            let mut sorted_mapping: Vec<_> = mapping.iter().collect();
            sorted_mapping.sort_by_key(|(src, _)| *src);
            for (src, dst) in sorted_mapping {
                let v = get_nested(input, src).unwrap_or(Value::Null);
                set_nested(&mut normalized, dst, v);
            }
            // Transform
            match &step.transform {
                Transform::Arithmetic(ar) => {
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
                Transform::Concat(ct) => {
                    let left = eval_string_operand(&normalized, &ct.left).unwrap_or_default();
                    let right = eval_string_operand(&normalized, &ct.right).unwrap_or_default();
                    let sep = ct.separator.clone().unwrap_or_default();
                    let combined = if sep.is_empty() {
                        format!("{}{}", left, right)
                    } else {
                        format!("{}{}{}", left, sep, right)
                    };
                    set_nested(&mut normalized, &ct.target, Value::from(combined));
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

fn eval_string_operand(ctx: &Value, op: &StringOperand) -> Option<String> {
    match op {
        StringOperand::Field { field } => {
            get_nested(ctx, field).and_then(|v| v.as_str().map(|s| s.to_string()))
        }
        StringOperand::ConstString { value } => Some(value.clone()),
    }
}

pub(crate) fn validate_mapping(
    idx: usize,
    mapping: &std::collections::HashMap<String, String>,
    safe_field: &Regex,
) -> anyhow::Result<()> {
    // Allow empty mappings
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
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example/csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": { "price": "price" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "price",
                    "left": { "kind": "field", "field": "price" },
                    "op": "add",
                    "right": { "kind": "const", "value": 5.0 }
                },
                // Mapping structure: { destination_field: normalized_field }
                // So "entity.total" (destination) maps from normalized "price"
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": { "entity.total": "price" }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({ "price": 10.0 });
        let out = prog.apply(&input).unwrap();
        assert_eq!(out["entity"]["total"], json!(15.0));
    }

    // No validation failure on empty mappings anymore

    #[test]
    fn test_mapping_destination_to_normalized() {
        // Test that mapping structure { destination_field: normalized_field } works correctly
        // This tests the fix where we swap (src, dst) to (dst, src) in the iteration
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "email": "email",
                        "active": "active",
                        "firstName": "firstName",
                        "lastName": "lastName"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "email": "email",
                        "published": "active",
                        "firstName": "firstName",
                        "lastName": "lastName",
                        "entity_key": "email"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        // Input has: email, active, firstName, lastName
        let input = json!({
            "email": "test@example.com",
            "active": true,
            "firstName": "John",
            "lastName": "Doe"
        });

        // Execute should produce: email, published (from active), firstName, lastName, entity_key (from email)
        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);

        let (to_def, produced) = &outputs[0];
        match to_def {
            ToDef::Entity { .. } => {
                // Verify that "active" was mapped to "published"
                assert_eq!(produced["published"], json!(true));
                // Verify that "email" was mapped to both "email" and "entity_key"
                assert_eq!(produced["email"], json!("test@example.com"));
                assert_eq!(produced["entity_key"], json!("test@example.com"));
                // Verify that "active" is NOT in the output (should be "published" instead)
                assert!(!produced.as_object().unwrap().contains_key("active"));
                // Verify other fields
                assert_eq!(produced["firstName"], json!("John"));
                assert_eq!(produced["lastName"], json!("Doe"));
            }
            _ => panic!("Expected Entity ToDef"),
        }
    }

    #[test]
    fn test_validate_from_api_without_endpoint() {
        // from.api without endpoint field should be valid (accepts POST)
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();
    }

    #[test]
    fn test_validate_from_api_with_endpoint_fails() {
        // from.api with endpoint field should fail validation
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {
                            "endpoint": "/api/v1/workflows/test"
                        },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        let result = prog.validate();
        assert!(
            result.is_err(),
            "Expected validation to fail for from.api with endpoint field"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("endpoint is not allowed"),
            "Error message should mention endpoint is not allowed"
        );
    }

    #[test]
    fn test_mapping_same_field_multiple_times() {
        // Test that the same normalized field can be mapped to multiple destination fields
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "email": "email"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "email": "email",
                        "entity_key": "email"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({
            "email": "test@example.com"
        });

        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);

        let (_, produced) = &outputs[0];
        // Both email and entity_key should have the same value
        assert_eq!(produced["email"], json!("test@example.com"));
        assert_eq!(produced["entity_key"], json!("test@example.com"));
    }

    #[test]
    fn test_mapping_apply_consistency() {
        // Test that apply() method has the same behavior as execute() for mapping
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "active": "active"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "published": "active"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({
            "active": true
        });

        // Test apply()
        let out = prog.apply(&input).unwrap();
        assert_eq!(out["published"], json!(true));
        assert!(!out.as_object().unwrap().contains_key("active"));

        // Test execute() - should produce same result
        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);
        let (_, produced) = &outputs[0];
        assert_eq!(produced["published"], json!(true));
        assert!(!produced.as_object().unwrap().contains_key("active"));
    }
}
