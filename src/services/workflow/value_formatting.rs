use crate::entity::field::types::FieldType;
use crate::entity::EntityDefinition;
use serde_json::Value;
use std::collections::HashMap;

/// Cast a JSON value to the appropriate type based on field definition
pub fn cast_field_value(value: Value, field_type: &FieldType) -> Value {
    match field_type {
        FieldType::Boolean => cast_to_boolean(value),
        FieldType::Integer => cast_to_integer(value),
        FieldType::Float => cast_to_float(value),
        _ => value, // For other types, keep as-is
    }
}

/// Cast a value to boolean
fn cast_to_boolean(value: Value) -> Value {
    match value {
        Value::Bool(b) => Value::Bool(b),
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Value::Bool(true),
            "false" | "no" | "0" | "off" | "" => Value::Bool(false),
            _ => {
                log::warn!(
                    "Cannot convert string '{}' to boolean, defaulting to false",
                    s
                );
                Value::Bool(false)
            }
        },
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Bool(i != 0)
            } else if let Some(f) = n.as_f64() {
                Value::Bool(f != 0.0)
            } else {
                Value::Bool(false)
            }
        }
        Value::Null => Value::Null,
        _ => {
            log::warn!("Cannot convert {:?} to boolean, defaulting to false", value);
            Value::Bool(false)
        }
    }
}

/// Cast a value to integer
fn cast_to_integer(value: Value) -> Value {
    match &value {
        Value::Number(n) if n.is_i64() => value,
        Value::Number(n) if n.is_u64() => {
            if let Some(i) = n.as_i64() {
                Value::Number(serde_json::Number::from(i))
            } else {
                value
            }
        }
        Value::String(s) => match s.parse::<i64>() {
            Ok(i) => Value::Number(serde_json::Number::from(i)),
            Err(_) => {
                log::warn!("Cannot convert string '{}' to integer", s);
                value
            }
        },
        Value::Null => Value::Null,
        _ => value,
    }
}

/// Cast a value to float
fn cast_to_float(value: Value) -> Value {
    match &value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    Value::Number(num)
                } else {
                    value
                }
            } else {
                value
            }
        }
        Value::String(s) => match s.parse::<f64>() {
            Ok(f) => {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    Value::Number(num)
                } else {
                    value
                }
            }
            Err(_) => {
                log::warn!("Cannot convert string '{}' to float", s);
                value
            }
        },
        Value::Null => Value::Null,
        _ => value,
    }
}

/// Normalize field data by casting values to their proper types based on entity definition
pub fn normalize_field_data_by_type(
    field_data: &mut HashMap<String, Value>,
    entity_definition: &EntityDefinition,
) {
    for field_def in &entity_definition.fields {
        if let Some(value) = field_data.get_mut(&field_def.name) {
            let casted = cast_field_value(value.clone(), &field_def.field_type);
            *value = casted;
        }
    }
}

/// Coerce published field from string to boolean if needed
pub fn coerce_published_field(value: Value) -> Value {
    match value {
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" => Value::Bool(true),
            "false" | "0" => Value::Bool(false),
            _ => Value::String(s),
        },
        other => other,
    }
}

/// Normalize a path string (ensure it starts with /)
pub fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

/// Reserved field names that have special handling
pub const RESERVED_FIELDS: &[&str] = &[
    "uuid",
    "path",
    "parent_uuid",
    "entity_key",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
    "published",
    "version",
];

/// Check if a field name is reserved
pub fn is_reserved_field(field_name: &str) -> bool {
    RESERVED_FIELDS.contains(&field_name)
}

/// Fields that should be ignored from import payload (protected fields)
pub const PROTECTED_FIELDS: &[&str] = &["created_at", "created_by"];

/// Check if a field name is protected (should not be overwritten)
pub fn is_protected_field(field_name: &str) -> bool {
    PROTECTED_FIELDS.contains(&field_name)
}

/// Process reserved fields with special handling
pub fn process_reserved_field(
    field_name: &str,
    value: Value,
    normalized_data: &mut HashMap<String, Value>,
) -> bool {
    // Coerce published from string if needed
    if field_name == "published" {
        normalized_data.insert(field_name.to_string(), coerce_published_field(value));
        return true;
    }

    // Allow explicit mapping of entity_key
    if field_name == "entity_key" {
        normalized_data.insert(field_name.to_string(), value);
        return true;
    }

    // Ignore protected fields from import payload
    if is_protected_field(field_name) {
        return true; // Skip this field
    }

    // Keep other reserved fields like uuid, path, parent_uuid, updated_at, updated_by, version if provided
    normalized_data.insert(field_name.to_string(), value);
    true
}

/// Build normalized field data from raw field data, handling reserved fields and type casting
pub fn build_normalized_field_data(
    field_data: HashMap<String, Value>,
    _entity_definition: &EntityDefinition,
) -> HashMap<String, Value> {
    let mut normalized = HashMap::new();

    for (k, v) in field_data.into_iter() {
        if is_reserved_field(&k) {
            process_reserved_field(&k, v, &mut normalized);
            continue;
        }
        // Keep field names exactly as provided - validator will check exact match
        normalized.insert(k, v);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cast_to_boolean_from_bool() {
        assert_eq!(
            cast_field_value(json!(true), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!(false), &FieldType::Boolean),
            json!(false)
        );
    }

    #[test]
    fn test_cast_to_boolean_from_string() {
        assert_eq!(
            cast_field_value(json!("true"), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!("True"), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!("TRUE"), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!("yes"), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!("1"), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!("on"), &FieldType::Boolean),
            json!(true)
        );

        assert_eq!(
            cast_field_value(json!("false"), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!("False"), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!("no"), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!("0"), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!("off"), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!(""), &FieldType::Boolean),
            json!(false)
        );
    }

    #[test]
    fn test_cast_to_boolean_from_number() {
        assert_eq!(cast_field_value(json!(1), &FieldType::Boolean), json!(true));
        assert_eq!(
            cast_field_value(json!(0), &FieldType::Boolean),
            json!(false)
        );
        assert_eq!(
            cast_field_value(json!(42), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!(1.5), &FieldType::Boolean),
            json!(true)
        );
        assert_eq!(
            cast_field_value(json!(0.0), &FieldType::Boolean),
            json!(false)
        );
    }

    #[test]
    fn test_cast_to_boolean_from_null() {
        assert_eq!(
            cast_field_value(json!(null), &FieldType::Boolean),
            json!(null)
        );
    }

    #[test]
    fn test_cast_to_integer_from_number() {
        assert_eq!(cast_field_value(json!(42), &FieldType::Integer), json!(42));
        assert_eq!(cast_field_value(json!(0), &FieldType::Integer), json!(0));
        assert_eq!(
            cast_field_value(json!(-10), &FieldType::Integer),
            json!(-10)
        );
    }

    #[test]
    fn test_cast_to_integer_from_string() {
        assert_eq!(
            cast_field_value(json!("42"), &FieldType::Integer),
            json!(42)
        );
        assert_eq!(cast_field_value(json!("0"), &FieldType::Integer), json!(0));
        assert_eq!(
            cast_field_value(json!("-10"), &FieldType::Integer),
            json!(-10)
        );
    }

    #[test]
    fn test_cast_to_float_from_number() {
        assert_eq!(
            cast_field_value(json!(3.14), &FieldType::Float),
            json!(3.14)
        );
        assert_eq!(cast_field_value(json!(0.0), &FieldType::Float), json!(0.0));
        assert_eq!(
            cast_field_value(json!(-2.5), &FieldType::Float),
            json!(-2.5)
        );
    }

    #[test]
    fn test_cast_to_float_from_string() {
        assert_eq!(
            cast_field_value(json!("3.14"), &FieldType::Float),
            json!(3.14)
        );
        assert_eq!(
            cast_field_value(json!("0.0"), &FieldType::Float),
            json!(0.0)
        );
        assert_eq!(
            cast_field_value(json!("-2.5"), &FieldType::Float),
            json!(-2.5)
        );
    }

    #[test]
    fn test_coerce_published_field() {
        assert_eq!(coerce_published_field(json!("true")), json!(true));
        assert_eq!(coerce_published_field(json!("True")), json!(true));
        assert_eq!(coerce_published_field(json!("1")), json!(true));
        assert_eq!(coerce_published_field(json!("false")), json!(false));
        assert_eq!(coerce_published_field(json!("False")), json!(false));
        assert_eq!(coerce_published_field(json!("0")), json!(false));
        assert_eq!(coerce_published_field(json!(true)), json!(true));
        assert_eq!(coerce_published_field(json!(false)), json!(false));
        assert_eq!(coerce_published_field(json!("unknown")), json!("unknown"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/test"), "/test");
        assert_eq!(normalize_path("test"), "/test");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path(""), "/");
    }

    #[test]
    fn test_is_reserved_field() {
        assert!(is_reserved_field("uuid"));
        assert!(is_reserved_field("path"));
        assert!(is_reserved_field("published"));
        assert!(is_reserved_field("version"));
        assert!(!is_reserved_field("custom_field"));
        assert!(!is_reserved_field("name"));
    }

    #[test]
    fn test_is_protected_field() {
        assert!(is_protected_field("created_at"));
        assert!(is_protected_field("created_by"));
        assert!(!is_protected_field("updated_at"));
        assert!(!is_protected_field("uuid"));
    }

    #[test]
    fn test_process_reserved_field_published() {
        let mut normalized = HashMap::new();
        process_reserved_field("published", json!("true"), &mut normalized);
        assert_eq!(normalized.get("published"), Some(&json!(true)));
    }

    #[test]
    fn test_process_reserved_field_protected() {
        let mut normalized = HashMap::new();
        process_reserved_field("created_at", json!("2024-01-01"), &mut normalized);
        // Protected fields should be skipped
        assert!(!normalized.contains_key("created_at"));
    }

    #[test]
    fn test_build_normalized_field_data() {
        let mut field_data = HashMap::new();
        field_data.insert("name".to_string(), json!("Test"));
        field_data.insert("published".to_string(), json!("true"));
        field_data.insert("created_at".to_string(), json!("2024-01-01"));

        let def = EntityDefinition::new(
            "test".to_string(),
            "Test".to_string(),
            None,
            None,
            false,
            None,
            vec![],
            uuid::Uuid::now_v7(),
        );

        let normalized = build_normalized_field_data(field_data, &def);

        assert_eq!(normalized.get("name"), Some(&json!("Test")));
        assert_eq!(normalized.get("published"), Some(&json!(true)));
        // Protected field should be removed
        assert!(!normalized.contains_key("created_at"));
    }
}
