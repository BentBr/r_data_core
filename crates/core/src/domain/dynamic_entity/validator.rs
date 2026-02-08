#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use regex::Regex;
use serde_json::Value;
use time::{macros::format_description, Date, OffsetDateTime};
use uuid::Uuid;

use crate::entity_definition::definition::EntityDefinition;
use crate::error::Result;
use crate::field::{FieldDefinition, FieldType};

// Create a ValidationContext struct to encapsulate common validation parameters
pub struct ValidationContext<'a> {
    field_def: &'a FieldDefinition,
    field_name: &'a str,
    value: &'a Value,
}

impl<'a> ValidationContext<'a> {
    #[must_use]
    pub fn new(field_def: &'a FieldDefinition, value: &'a Value) -> Self {
        Self {
            field_def,
            field_name: &field_def.name,
            value,
        }
    }

    #[must_use]
    pub const fn with_field_name(
        field_def: &'a FieldDefinition,
        value: &'a Value,
        field_name: &'a str,
    ) -> Self {
        Self {
            field_def,
            field_name,
            value,
        }
    }

    #[must_use]
    pub fn create_validation_error(&self, message: &str) -> crate::error::Error {
        crate::error::Error::Validation(format!("Field '{}' {}", self.field_name, message))
    }

    /// # Errors
    /// Returns an error if validation fails
    pub fn validate_number_range(&self, num_value: f64) -> Result<()> {
        // Range validation
        if let Some(min_value) = &self.field_def.validation.min_value {
            let min = min_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid min_value"))?;
            if num_value < min {
                return Err(self.create_validation_error(&format!("must be at least {min}")));
            }
        }

        if let Some(max_value) = &self.field_def.validation.max_value {
            let max = max_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid max_value"))?;
            if num_value > max {
                return Err(self.create_validation_error(&format!("must be no more than {max}")));
            }
        }

        // Positive only validation
        if self.field_def.validation.positive_only == Some(true) && num_value < 0.0 {
            return Err(self.create_validation_error("must be a positive number"));
        }

        Ok(())
    }

    /// # Panics
    /// May panic if value is not a string when checking for empty strings
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub fn check_required(&self) -> Result<bool> {
        // Check if the field is required and the value is null or empty
        if self.field_def.required
            && (self.value.is_null()
                || (self.value.is_string() && self.value.as_str().unwrap().is_empty()))
        {
            return Err(self.create_validation_error("is required"));
        }

        // If the value is null and the field is not required, skip validation
        Ok(!self.value.is_null())
    }
}

/// Validator for dynamic entities
pub struct DynamicEntityValidator;

impl DynamicEntityValidator {
    /// Validate a field against its definition
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub fn validate_field(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        let ctx = ValidationContext::new(field_def, value);

        // Skip further validation if not required and null
        if !ctx.check_required()? {
            return Ok(());
        }

        // Validate based on a field type
        match field_def.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => Self::validate_string(&ctx),
            FieldType::Integer => Self::validate_integer(&ctx),
            FieldType::Float => Self::validate_float(&ctx),
            FieldType::Boolean => Self::validate_boolean(&ctx),
            FieldType::Date => Self::validate_date(&ctx),
            FieldType::DateTime => Self::validate_datetime(&ctx),
            FieldType::Uuid => Self::validate_uuid(&ctx),
            FieldType::Select => Self::validate_select(&ctx),
            FieldType::MultiSelect => Self::validate_multi_select(&ctx),
            FieldType::Array => Self::validate_array(&ctx),
            FieldType::Object => Self::validate_object(&ctx),
            // Json accepts any valid JSON value (objects, arrays, strings, numbers, booleans, null)
            // No additional validation needed since serde_json already ensures valid JSON
            FieldType::Json
            | FieldType::ManyToOne
            | FieldType::ManyToMany
            | FieldType::Image
            | FieldType::File => Ok(()),
        }
    }

    /// Validate string fields
    fn validate_string(ctx: &ValidationContext) -> Result<()> {
        if !ctx.value.is_string() {
            return Err(ctx.create_validation_error("must be a string"));
        }

        let string_value = ctx.value.as_str().unwrap();

        // Length validation
        if let Some(min_length) = ctx.field_def.validation.min_length {
            if string_value.len() < min_length {
                return Err(ctx.create_validation_error(&format!(
                    "must be at least {min_length} characters"
                )));
            }
        }

        if let Some(max_length) = ctx.field_def.validation.max_length {
            if string_value.len() > max_length {
                return Err(ctx.create_validation_error(&format!(
                    "must be no more than {max_length} characters"
                )));
            }
        }

        // Pattern validation
        if let Some(pattern) = &ctx.field_def.validation.pattern {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(string_value) {
                        return Err(
                            ctx.create_validation_error(&format!("must match pattern: {pattern}"))
                        );
                    }
                }
                Err(_) => {
                    return Err(ctx.create_validation_error(&format!(
                        "has invalid regex pattern: {pattern}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate integer fields
    fn validate_integer(ctx: &ValidationContext) -> Result<()> {
        let int_value = match ctx.value {
            Value::Number(n) if n.is_i64() => n.as_i64().unwrap(),
            Value::Number(n) if n.is_u64() => n.as_u64().unwrap().try_into().unwrap_or(i64::MAX),
            Value::String(s) => s
                .parse::<i64>()
                .map_err(|_| ctx.create_validation_error("must be a valid integer"))?,
            _ => {
                return Err(ctx.create_validation_error("must be an integer"));
            }
        };

        #[allow(clippy::cast_precision_loss)] // i64 to f64 conversion for validation
        ctx.validate_number_range(int_value as f64)?;

        Ok(())
    }

    /// Validate float fields
    fn validate_float(ctx: &ValidationContext) -> Result<()> {
        let float_value = match ctx.value {
            Value::Number(n) => n.as_f64().unwrap(),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| ctx.create_validation_error("must be a valid number"))?,
            _ => {
                return Err(ctx.create_validation_error("must be a number"));
            }
        };

        ctx.validate_number_range(float_value)?;

        Ok(())
    }

    /// Validate boolean fields
    fn validate_boolean(ctx: &ValidationContext) -> Result<()> {
        match ctx.value {
            Value::Bool(_) => Ok(()),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "false" | "no" | "0" => Ok(()),
                _ => Err(ctx.create_validation_error("must be a boolean value")),
            },
            Value::Number(n) => {
                if n.as_i64() == Some(0) || n.as_i64() == Some(1) {
                    Ok(())
                } else {
                    Err(ctx.create_validation_error("must be a boolean value (0 or 1)"))
                }
            }
            _ => Err(ctx.create_validation_error("must be a boolean")),
        }
    }

    /// Validate array fields
    fn validate_array(ctx: &ValidationContext) -> Result<()> {
        if !ctx.value.is_array() {
            return Err(ctx.create_validation_error("must be an array"));
        }
        Ok(())
    }

    /// Validate object fields
    fn validate_object(ctx: &ValidationContext) -> Result<()> {
        if !ctx.value.is_object() {
            return Err(ctx.create_validation_error("must be an object"));
        }
        Ok(())
    }

    /// Validate date fields
    fn validate_date(ctx: &ValidationContext) -> Result<()> {
        let Value::String(date_str) = ctx.value else {
            return Err(ctx.create_validation_error("must be a date string"));
        };

        // Format description for YYYY-MM-DD
        let format = format_description!("[year]-[month]-[day]");

        let date = Date::parse(date_str, &format).map_err(|_| {
            ctx.create_validation_error("must be a valid date in YYYY-MM-DD format")
        })?;

        // Date range validation
        let now = OffsetDateTime::now_utc().date();

        if let Some(min_date_str) = &ctx.field_def.validation.min_date {
            let min_date = if min_date_str == "now" {
                now
            } else {
                Date::parse(min_date_str, &format)
                    .map_err(|_| ctx.create_validation_error("Invalid min_date format"))?
            };

            if date < min_date {
                return Err(ctx.create_validation_error(&format!("must be on or after {min_date}")));
            }
        }

        if let Some(max_date_str) = &ctx.field_def.validation.max_date {
            let max_date = if max_date_str == "now" {
                now
            } else {
                Date::parse(max_date_str, &format)
                    .map_err(|_| ctx.create_validation_error("Invalid max_date format"))?
            };

            if date > max_date {
                return Err(
                    ctx.create_validation_error(&format!("must be on or before {max_date}"))
                );
            }
        }

        Ok(())
    }

    /// Validate datetime fields
    fn validate_datetime(ctx: &ValidationContext) -> Result<()> {
        let Value::String(datetime_str) = ctx.value else {
            return Err(ctx.create_validation_error("must be a datetime string"));
        };

        // Parse ISO8601 / RFC3339 datetime
        let datetime =
            OffsetDateTime::parse(datetime_str, &time::format_description::well_known::Rfc3339)
                .map_err(|_| {
                    ctx.create_validation_error("must be a valid datetime in RFC3339 format")
                })?;

        // Datetime range validation
        let now = OffsetDateTime::now_utc();

        if let Some(min_date_str) = &ctx.field_def.validation.min_date {
            let min_date = if min_date_str == "now" {
                now
            } else {
                OffsetDateTime::parse(min_date_str, &time::format_description::well_known::Rfc3339)
                    .map_err(|_| ctx.create_validation_error("Invalid min_date format"))?
            };

            if datetime < min_date {
                return Err(ctx.create_validation_error(&format!("must be on or after {min_date}")));
            }
        }

        if let Some(max_date_str) = &ctx.field_def.validation.max_date {
            let max_date = if max_date_str == "now" {
                now
            } else {
                OffsetDateTime::parse(max_date_str, &time::format_description::well_known::Rfc3339)
                    .map_err(|_| ctx.create_validation_error("Invalid max_date format"))?
            };

            if datetime > max_date {
                return Err(
                    ctx.create_validation_error(&format!("must be on or before {max_date}"))
                );
            }
        }

        Ok(())
    }

    /// Validate UUID fields
    fn validate_uuid(ctx: &ValidationContext) -> Result<()> {
        let Value::String(uuid_str) = ctx.value else {
            return Err(ctx.create_validation_error("must be a UUID string"));
        };

        Uuid::parse_str(uuid_str)
            .map_err(|_| ctx.create_validation_error("must be a valid UUID"))?;

        Ok(())
    }

    /// Validate select fields
    fn validate_select(ctx: &ValidationContext) -> Result<()> {
        let Value::String(option_value) = ctx.value else {
            return Err(ctx.create_validation_error("must be a string"));
        };

        // Validate against options if present
        if let Some(crate::field::OptionsSource::Fixed { options }) =
            &ctx.field_def.validation.options_source
        {
            let valid_options: Vec<String> = options.iter().map(|opt| opt.value.clone()).collect();

            if !valid_options.contains(option_value) {
                return Err(ctx.create_validation_error(&format!(
                    "must be one of: {}",
                    valid_options.join(", ")
                )));
            }
        }

        Ok(())
    }

    /// Validate multi-select fields
    fn validate_multi_select(ctx: &ValidationContext) -> Result<()> {
        let selected_values = match ctx.value {
            Value::Array(arr) => arr
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.clone()),
                    _ => Err(ctx.create_validation_error("must contain only strings")),
                })
                .collect::<Result<Vec<String>>>()?,
            Value::String(s) => vec![s.clone()],
            _ => {
                return Err(ctx.create_validation_error("must be an array of strings"));
            }
        };

        // Validate against options if present
        if let Some(crate::field::OptionsSource::Fixed { options }) =
            &ctx.field_def.validation.options_source
        {
            let valid_options: Vec<String> = options.iter().map(|opt| opt.value.clone()).collect();

            for value in &selected_values {
                if !valid_options.contains(value) {
                    return Err(ctx.create_validation_error(&format!(
                        "contains invalid option '{value}'. Valid options are: {}",
                        valid_options.join(", ")
                    )));
                }
            }
        }

        Ok(())
    }
}

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

/// Represents a field-specific validation error
#[derive(Debug, Clone)]
pub struct FieldViolation {
    pub field: String,
    pub message: String,
}

/// # Errors
/// Returns an error if validation fails
pub fn validate_entity(entity: &Value, entity_def: &EntityDefinition) -> Result<()> {
    let violations = validate_entity_with_violations(entity, entity_def)?;
    if !violations.is_empty() {
        return Err(crate::error::Error::Validation(format!(
            "Validation failed with the following errors: {}",
            violations
                .iter()
                .map(|v| format!("Field '{}': {}", v.field, v.message))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    Ok(())
}

/// Validate entity and return structured violations
///
/// # Errors
/// Returns an error if validation fails
pub fn validate_entity_with_violations(
    entity: &Value,
    entity_def: &EntityDefinition,
) -> Result<Vec<FieldViolation>> {
    let mut violations = Vec::new();
    let entity_type = entity
        .get("entity_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::Error::Validation("Entity must have an entity_type field".to_string())
        })?;

    if entity_type != entity_def.entity_type {
        return Err(crate::error::Error::Validation(format!(
            "Entity type '{}' does not match entity definition type '{}'",
            entity_type, entity_def.entity_type
        )));
    }

    let field_data = entity
        .get("field_data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            crate::error::Error::Validation("Entity must have a field_data object".to_string())
        })?;

    // Check required fields
    for field_def in &entity_def.fields {
        if field_def.required && !field_data.contains_key(&field_def.name) {
            violations.push(FieldViolation {
                field: field_def.name.clone(),
                message: "This field is required".to_string(),
            });
        }
    }

    // Validate fields that are present
    for (field_name, value) in field_data {
        if let Some(field_def) = entity_def.get_field(field_name) {
            let _ = ValidationContext::with_field_name(field_def, value, field_name);
            if let Err(e) = DynamicEntityValidator::validate_field(field_def, value) {
                // Extract just the inner message from the Error::Validation variant
                // and strip the "Field 'x' " prefix if present for cleaner violation messages
                let message = match e {
                    crate::error::Error::Validation(msg) => {
                        // Strip "Field 'field_name' " prefix if present
                        let prefix = format!("Field '{field_name}' ");
                        msg.strip_prefix(&prefix).unwrap_or(&msg).to_string()
                    }
                    other => other.to_string(),
                };
                violations.push(FieldViolation {
                    field: field_name.clone(),
                    message,
                });
            }
        } else {
            // Skip system fields
            let system_fields = [
                "uuid",
                "entity_key",
                "path",
                "created_at",
                "updated_at",
                "created_by",
                "updated_by",
                "published",
                "version",
                "parent_uuid", // Parent entity reference
            ];
            if !system_fields.contains(&field_name.as_str()) {
                violations.push(FieldViolation {
                    field: field_name.clone(),
                    message: "This field is not defined in the entity definition".to_string(),
                });
            }
        }
    }

    Ok(violations)
}

/// Validate that `parent_uuid` and path are consistent
/// Returns Ok(()) if valid, or adds violations if invalid
/// This function checks the relationship between `parent_uuid` and path
///
/// # Errors
/// Returns an error if validation processing fails (should not happen in normal operation).
pub fn validate_parent_path_consistency(
    parent_uuid: Option<String>,
    path: Option<&String>,
    expected_path: Option<&String>,
) -> Result<Vec<FieldViolation>> {
    let mut violations = Vec::new();

    // If parent_uuid is set, we need to validate the path
    if let Some(parent_uuid_str) = parent_uuid {
        if !parent_uuid_str.is_empty() {
            // If we have an expected path (from parent entity), validate it
            if let Some(expected) = &expected_path {
                if let Some(actual_path) = &path {
                    if actual_path != expected {
                        violations.push(FieldViolation {
                            field: "path".to_string(),
                            message: format!(
                                "Path must match parent's path + key. Expected: {expected}, got: {actual_path}"
                            ),
                        });
                    }
                } else {
                    violations.push(FieldViolation {
                        field: "path".to_string(),
                        message: "Path is required when parent_uuid is set".to_string(),
                    });
                }
            }
        }
    }

    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::ui::UiSettings;
    use serde_json::json;

    fn create_test_field(field_type: FieldType, required: bool) -> FieldDefinition {
        FieldDefinition {
            name: "test_field".to_string(),
            display_name: "Test Field".to_string(),
            field_type,
            required,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: crate::field::FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: std::collections::HashMap::new(),
            description: None,
        }
    }

    fn create_named_field(
        name: &str,
        display_name: &str,
        field_type: FieldType,
    ) -> FieldDefinition {
        FieldDefinition {
            name: name.to_string(),
            display_name: display_name.to_string(),
            field_type,
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: crate::field::FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: std::collections::HashMap::new(),
            description: None,
        }
    }

    mod validate_json_tests {
        use super::*;

        #[test]
        fn test_validate_json_accepts_object() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!({"key": "value", "nested": {"inner": 1}});
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_string() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!("a plain string value");
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_array() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!([1, 2, 3]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_array_of_objects() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!([
                {"entity_type": "Customer", "count": 100},
                {"entity_type": "Order", "count": 500}
            ]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_number() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!(123);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_boolean() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!(true);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_allows_null_when_not_required() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!(null);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_rejects_null_when_required() {
            let field_def = create_test_field(FieldType::Json, true);
            let value = json!(null);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("is required"));
        }

        #[test]
        fn test_validate_json_accepts_complex_nested_object() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!({
                "order_items": {
                    "0": "product1",
                    "1": "product2"
                },
                "metadata": {
                    "count": 2,
                    "tags": ["tag1", "tag2"]
                }
            });
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_empty_object() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!({});
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_json_accepts_empty_array() {
            let field_def = create_test_field(FieldType::Json, false);
            let value = json!([]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }
    }

    mod validate_object_tests {
        use super::*;

        #[test]
        fn test_validate_object_accepts_object() {
            let field_def = create_test_field(FieldType::Object, false);
            let value = json!({"key": "value"});
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_object_rejects_string() {
            let field_def = create_test_field(FieldType::Object, false);
            let value = json!("not an object");
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("must be an object"));
        }

        #[test]
        fn test_validate_object_rejects_array() {
            let field_def = create_test_field(FieldType::Object, false);
            let value = json!([1, 2, 3]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_err());
        }
    }

    mod validate_array_tests {
        use super::*;

        #[test]
        fn test_validate_array_accepts_array() {
            let field_def = create_test_field(FieldType::Array, false);
            let value = json!([1, 2, 3]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_array_accepts_array_of_objects() {
            let field_def = create_test_field(FieldType::Array, false);
            let value = json!([{"id": 1}, {"id": 2}]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_array_rejects_object() {
            let field_def = create_test_field(FieldType::Array, false);
            let value = json!({"key": "value"});
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("must be an array"));
        }

        #[test]
        fn test_validate_array_rejects_string() {
            let field_def = create_test_field(FieldType::Array, false);
            let value = json!("[1, 2, 3]"); // String representation of array
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_array_accepts_empty_array() {
            let field_def = create_test_field(FieldType::Array, false);
            let value = json!([]);
            let result = DynamicEntityValidator::validate_field(&field_def, &value);
            assert!(result.is_ok());
        }
    }

    mod validate_entity_with_violations_tests {
        use super::*;
        use crate::entity_definition::definition::EntityDefinition;

        fn create_test_entity_definition() -> EntityDefinition {
            EntityDefinition {
                entity_type: "test_entity".to_string(),
                display_name: "Test Entity".to_string(),
                fields: vec![
                    create_test_field(FieldType::String, false),
                    create_named_field("json_content", "JSON Content", FieldType::Json),
                    create_named_field("items", "Items", FieldType::Array),
                ],
                published: true,
                ..EntityDefinition::default()
            }
        }

        #[test]
        fn test_json_field_accepts_string_value() {
            // Json field type now accepts any valid JSON value including strings
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "json_content": "a plain string value"
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert!(
                violations.is_empty(),
                "Json field should accept string values"
            );
        }

        #[test]
        fn test_json_field_accepts_array_value() {
            // Json field type now accepts any valid JSON value including arrays
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "json_content": [{"key": "value"}, {"key": "value2"}]
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert!(
                violations.is_empty(),
                "Json field should accept array values"
            );
        }

        #[test]
        fn test_violations_for_array_string_value() {
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "items": "[1, 2, 3]" // String instead of array
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert_eq!(violations.len(), 1);
            assert_eq!(violations[0].field, "items");
            assert!(violations[0].message.contains("must be an array"));
        }

        #[test]
        fn test_no_violations_for_valid_json_object() {
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "json_content": {"key": "value", "count": 5}
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert!(violations.is_empty());
        }

        #[test]
        fn test_no_violations_for_valid_array() {
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "items": [{"id": 1}, {"id": 2}]
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert!(violations.is_empty());
        }

        #[test]
        fn test_violations_for_invalid_array_field() {
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "json_content": "valid json string value",  // Json accepts any value
                    "items": "not an array"  // Array field requires array
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            // Only items field should have a violation (json_content accepts any value now)
            assert_eq!(violations.len(), 1);
            assert!(violations
                .iter()
                .map(|v| v.field.as_str())
                .any(|x| x == "items"));
        }

        #[test]
        fn test_violation_message_format_for_array() {
            let entity_def = create_test_entity_definition();
            let entity = json!({
                "entity_type": "test_entity",
                "field_data": {
                    "items": "not an array"  // String instead of array
                }
            });

            let result = validate_entity_with_violations(&entity, &entity_def);
            assert!(result.is_ok());
            let violations = result.unwrap();
            assert_eq!(violations.len(), 1);

            // Message should be clean without the redundant "Field 'x'" prefix
            // since the field name is in the separate 'field' property
            let msg = &violations[0].message;
            assert_eq!(msg, "must be an array");
        }
    }
}
