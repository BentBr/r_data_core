#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

use crate::error::Result;

/// # Errors
/// Returns an error if validation fails
pub fn validate_field(field_def: &Value, value: &Value, field_name: &str) -> Result<()> {
    let field_type = field_def
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            crate::error::Error::Validation(format!("Missing type for field {field_name}"))
        })?;

    match field_type {
        "string" => {
            if !value.is_string() {
                return Err(crate::error::Error::Validation(format!(
                    "Field {field_name} must be a string"
                )));
            }
            Ok(())
        }
        "number" | "integer" => {
            if !value.is_number() {
                return Err(crate::error::Error::Validation(format!(
                    "Field {field_name} must be a number"
                )));
            }
            Ok(())
        }
        "boolean" => {
            if !value.is_boolean() {
                return Err(crate::error::Error::Validation(format!(
                    "Field {field_name} must be a boolean"
                )));
            }
            Ok(())
        }
        "array" => {
            if !value.is_array() {
                return Err(crate::error::Error::Validation(format!(
                    "Field {field_name} must be an array"
                )));
            }
            Ok(())
        }
        "object" => {
            if !value.is_object() {
                return Err(crate::error::Error::Validation(format!(
                    "Field {field_name} must be an object"
                )));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
