use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::{Error, Result};

/// Our own Value enum that can be converted to/from serde_json::Value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

/// Trait for converting to serde_json::Value
pub trait ToValue {
    fn to_value(&self) -> Result<JsonValue>;
}

/// Trait for converting from serde_json::Value
pub trait FromValue: Sized {
    fn from_value(value: &JsonValue) -> Result<Self>;
}

// Implement FromValue for common types
impl FromValue for String {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            JsonValue::Number(n) => Ok(n.to_string()),
            JsonValue::Bool(b) => Ok(b.to_string()),
            JsonValue::Null => Ok(String::new()),
            _ => Err(Error::FieldConversion("string".to_string(), format!("Cannot convert {:?} to string", value))),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Number(n) => n.as_i64().ok_or_else(|| Error::FieldConversion("i64".to_string(), "number cannot be represented as i64".to_string())),
            JsonValue::String(s) => s.parse::<i64>().map_err(|e| Error::FieldConversion("i64".to_string(), e.to_string())),
            _ => Err(Error::FieldConversion("i64".to_string(), format!("Cannot convert {:?} to i64", value))),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Number(n) => n.as_f64().ok_or_else(|| Error::FieldConversion("f64".to_string(), "error converting to f64".to_string())),
            JsonValue::String(s) => s.parse::<f64>().map_err(|e| Error::FieldConversion("f64".to_string(), e.to_string())),
            _ => Err(Error::FieldConversion("f64".to_string(), format!("Cannot convert {:?} to f64", value))),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Bool(b) => Ok(*b),
            JsonValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(true),
                "false" | "no" | "0" => Ok(false),
                _ => Err(Error::FieldConversion("bool".to_string(), format!("invalid boolean string '{}'", s))),
            },
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i != 0)
                } else {
                    Err(Error::FieldConversion("bool".to_string(), "number cannot be converted to boolean".to_string()))
                }
            },
            _ => Err(Error::FieldConversion("bool".to_string(), format!("Cannot convert {:?} to bool", value))),
        }
    }
}

impl FromValue for Uuid {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => Uuid::parse_str(s).map_err(|e| Error::FieldConversion("uuid".to_string(), e.to_string())),
            _ => Err(Error::FieldConversion("uuid".to_string(), format!("Cannot convert {:?} to UUID", value))),
        }
    }
}

impl FromValue for DateTime<Utc> {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| Error::FieldConversion("datetime".to_string(), e.to_string())),
            _ => Err(Error::FieldConversion("datetime".to_string(), format!("Cannot convert {:?} to DateTime", value))),
        }
    }
}

// Implement ToValue for common types
impl ToValue for String {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::String(self.clone()))
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::Number((*self).into()))
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::Number(serde_json::Number::from_f64(*self).ok_or_else(|| Error::FieldConversion("f64".to_string(), "unable to represent as JSON number".to_string()))?))
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::Bool(*self))
    }
}

impl ToValue for Uuid {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::String(self.to_string()))
    }
}

impl ToValue for DateTime<Utc> {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::String(self.to_rfc3339()))
    }
}

// Implement FromValue for Vec<T> where T: FromValue
impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: &JsonValue) -> Result<Self> {
        if let JsonValue::Array(array) = value {
            let mut result = Vec::with_capacity(array.len());
            for item in array {
                result.push(T::from_value(item)?);
            }
            Ok(result)
        } else {
            Err(Error::FieldConversion("array".to_string(), format!("Cannot convert {:?} to array", value)))
        }
    }
}

// Implement ToValue for Vec<T> where T: ToValue
impl<T: ToValue> ToValue for Vec<T> {
    fn to_value(&self) -> Result<JsonValue> {
        let mut result = Vec::with_capacity(self.len());
        for item in self {
            result.push(item.to_value()?);
        }
        Ok(JsonValue::Array(result))
    }
}

// Implement FromValue for HashMap<String, T> where T: FromValue
impl<T: FromValue> FromValue for HashMap<String, T> {
    fn from_value(value: &JsonValue) -> Result<Self> {
        if let JsonValue::Object(obj) = value {
            let mut result = HashMap::with_capacity(obj.len());
            for (key, value) in obj {
                result.insert(key.clone(), T::from_value(value)?);
            }
            Ok(result)
        } else {
            Err(Error::FieldConversion("object".to_string(), format!("Cannot convert {:?} to object", value)))
        }
    }
}

// Implement ToValue for HashMap<String, T> where T: ToValue
impl<T: ToValue> ToValue for HashMap<String, T> {
    fn to_value(&self) -> Result<JsonValue> {
        let mut result = serde_json::Map::with_capacity(self.len());
        for (key, value) in self {
            result.insert(key.clone(), value.to_value()?);
        }
        Ok(JsonValue::Object(result))
    }
}

/// Implement FromValue for Option<T> where T: FromValue
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: &JsonValue) -> Result<Self> {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(T::from_value(value)?))
        }
    }
}

/// Implement ToValue for Option<T> where T: ToValue
impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Result<JsonValue> {
        match self {
            Some(value) => value.to_value(),
            None => Ok(JsonValue::Null),
        }
    }
} 