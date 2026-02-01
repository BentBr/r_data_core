#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

use super::transform::{Operand, StringOperand};

/// Cast a JSON value to f64 with strict error handling
///
/// # Arguments
/// * `value` - The JSON value to cast
/// * `field_name` - The field name for error messages
///
/// # Returns
/// Result with f64 or error message
///
/// # Errors
/// Returns an error if the value cannot be converted to f64
pub fn cast_to_f64_strict(value: &Value, field_name: &str) -> Result<f64, String> {
    match value {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| format!("Field '{field_name}': number out of f64 range")),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(format!(
                    "Field '{field_name}': empty string cannot be converted to number"
                ));
            }
            trimmed
                .parse::<f64>()
                .map_err(|_| format!("Field '{field_name}': cannot convert string '{s}' to number"))
        }
        Value::Null => Err(format!("Field '{field_name}' is null, expected a number")),
        Value::Bool(_) => Err(format!(
            "Field '{field_name}' is boolean, expected a number"
        )),
        Value::Array(_) => Err(format!(
            "Field '{field_name}' is an array, expected a number"
        )),
        Value::Object(_) => Err(format!(
            "Field '{field_name}' is an object, expected a number"
        )),
    }
}

/// Cast a JSON value to string with smart number formatting
/// - Integers and integer-valued floats: "123" not "123.0"
/// - Decimals: "123.45"
/// - Booleans: "true" or "false"
///
/// # Arguments
/// * `value` - The JSON value to cast
/// * `field_name` - The field name for error messages
///
/// # Returns
/// Result with String or error message
///
/// # Errors
/// Returns an error if the value cannot be converted to string
pub fn cast_to_string_smart(value: &Value, field_name: &str) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => n.as_i64().map_or_else(
            || {
                n.as_f64().map_or_else(
                    || Ok(n.to_string()), // Fallback for u64 or other number types
                    |f| {
                        // Check if float is effectively an integer
                        #[allow(clippy::float_cmp)] // We explicitly want exact comparison here
                        if f.fract() == 0.0 && f.is_finite() {
                            Ok(format!("{f:.0}"))
                        } else {
                            Ok(f.to_string())
                        }
                    },
                )
            },
            |i| Ok(i.to_string()),
        ),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Err(format!(
            "Field '{field_name}' is null, cannot convert to string"
        )),
        Value::Array(_) => Err(format!(
            "Field '{field_name}' is an array, cannot convert to string"
        )),
        Value::Object(_) => Err(format!(
            "Field '{field_name}' is an object, cannot convert to string"
        )),
    }
}

/// Evaluate a numeric operand with strict type casting
///
/// # Arguments
/// * `ctx` - Context JSON value
/// * `op` - Operand to evaluate
///
/// # Returns
/// Result with f64 value or error message
///
/// # Errors
/// Returns an error if the operand cannot be evaluated or cast to a number
pub fn eval_operand(ctx: &Value, op: &Operand) -> Result<f64, String> {
    match op {
        Operand::Field { field } => {
            let value = get_nested(ctx, field)
                .ok_or_else(|| format!("Field '{field}' not found in context"))?;
            cast_to_f64_strict(&value, field)
        }
        Operand::Const { value } => Ok(*value),
        Operand::ExternalEntityField { .. } => {
            // Future: resolve from repository; for now not supported
            Err("ExternalEntityField is not supported in calculations".to_string())
        }
    }
}

/// Evaluate a string operand with smart type casting
///
/// # Arguments
/// * `ctx` - Context JSON value
/// * `op` - String operand to evaluate
///
/// # Returns
/// Result with String value or error message
///
/// # Errors
/// Returns an error if the operand cannot be evaluated or cast to a string
pub fn eval_string_operand(ctx: &Value, op: &StringOperand) -> Result<String, String> {
    match op {
        StringOperand::Field { field } => {
            let value = get_nested(ctx, field)
                .ok_or_else(|| format!("Field '{field}' not found in context"))?;
            cast_to_string_smart(&value, field)
        }
        StringOperand::ConstString { value } => Ok(value.clone()),
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

/// Prefix for literal values in mapping
pub const LITERAL_PREFIX: &str = "@literal:";

/// Parse a literal value from a mapping source string
///
/// Literal values use the `@literal:` prefix followed by a JSON value.
/// This allows setting constant values in entity mappings without reading from input data.
///
/// # Examples
/// - `@literal:true` → `Value::Bool(true)`
/// - `@literal:false` → `Value::Bool(false)`
/// - `@literal:"string"` → `Value::String("string")`
/// - `@literal:123` → `Value::Number(123)`
/// - `@literal:null` → `Value::Null`
///
/// # Arguments
/// * `source` - The source string from mapping (e.g., `@literal:true` or `field_name`)
///
/// # Returns
/// `Some(Value)` if the source is a literal value, `None` if it's a field reference
pub fn parse_literal_value(source: &str) -> Option<Value> {
    if !source.starts_with(LITERAL_PREFIX) {
        return None;
    }

    let json_str = &source[LITERAL_PREFIX.len()..];

    // Parse the JSON value
    serde_json::from_str(json_str).ok()
}

#[cfg(test)]
mod literal_value_tests {
    use super::*;

    #[test]
    fn test_literal_true() {
        let result = parse_literal_value("@literal:true");
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_literal_false() {
        let result = parse_literal_value("@literal:false");
        assert_eq!(result, Some(Value::Bool(false)));
    }

    #[test]
    fn test_literal_string() {
        let result = parse_literal_value("@literal:\"hello world\"");
        assert_eq!(result, Some(Value::String("hello world".to_string())));
    }

    #[test]
    fn test_literal_number_integer() {
        let result = parse_literal_value("@literal:42");
        assert_eq!(result, Some(serde_json::json!(42)));
    }

    #[test]
    fn test_literal_number_float() {
        let result = parse_literal_value("@literal:3.5");
        assert_eq!(result, Some(serde_json::json!(3.5)));
    }

    #[test]
    fn test_literal_null() {
        let result = parse_literal_value("@literal:null");
        assert_eq!(result, Some(Value::Null));
    }

    #[test]
    fn test_literal_array() {
        let result = parse_literal_value("@literal:[1, 2, 3]");
        assert_eq!(result, Some(serde_json::json!([1, 2, 3])));
    }

    #[test]
    fn test_literal_object() {
        let result = parse_literal_value("@literal:{\"key\": \"value\"}");
        assert_eq!(result, Some(serde_json::json!({"key": "value"})));
    }

    #[test]
    fn test_not_literal_returns_none() {
        let result = parse_literal_value("field_name");
        assert_eq!(result, None);
    }

    #[test]
    fn test_not_literal_with_at_sign() {
        let result = parse_literal_value("@field_name");
        assert_eq!(result, None);
    }

    #[test]
    fn test_invalid_json_returns_none() {
        let result = parse_literal_value("@literal:invalid");
        assert_eq!(result, None);
    }

    #[test]
    fn test_empty_literal_returns_none() {
        let result = parse_literal_value("@literal:");
        assert_eq!(result, None);
    }
}
