use regex::Regex;
use serde_json::Value;
use time::{macros::format_description, Date, OffsetDateTime};
use uuid::Uuid;

use crate::entity::field::{FieldDefinition, FieldType};
use crate::entity::EntityDefinition;
use crate::error::{Error, Result};

// Create a ValidationContext struct to encapsulate common validation parameters
pub struct ValidationContext<'a> {
    field_def: &'a FieldDefinition,
    field_name: &'a str,
    value: &'a Value,
}

impl<'a> ValidationContext<'a> {
    pub fn new(field_def: &'a FieldDefinition, value: &'a Value) -> Self {
        Self {
            field_def,
            field_name: &field_def.name,
            value,
        }
    }

    pub fn with_field_name(
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

    pub fn create_validation_error(&self, message: &str) -> Error {
        Error::Validation(format!("Field '{}' {}", self.field_name, message))
    }

    pub fn validate_number_range(&self, num_value: f64) -> Result<()> {
        // Range validation
        if let Some(min_value) = &self.field_def.validation.min_value {
            let min = min_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid min_value"))?;
            if num_value < min {
                return Err(self.create_validation_error(&format!("must be at least {}", min)));
            }
        }

        if let Some(max_value) = &self.field_def.validation.max_value {
            let max = max_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid max_value"))?;
            if num_value > max {
                return Err(self.create_validation_error(&format!("must be no more than {}", max)));
            }
        }

        // Positive only validation
        if let Some(true) = self.field_def.validation.positive_only {
            if num_value < 0.0 {
                return Err(self.create_validation_error("must be a positive number"));
            }
        }

        Ok(())
    }

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
            FieldType::Json => Self::validate_json(&ctx),
            FieldType::ManyToOne | FieldType::ManyToMany => Ok(()), // Relation validation is handled separately
            FieldType::Image | FieldType::File => Ok(()), // Asset validation is handled separately
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
                    "must be at least {} characters",
                    min_length
                )));
            }
        }

        if let Some(max_length) = ctx.field_def.validation.max_length {
            if string_value.len() > max_length {
                return Err(ctx.create_validation_error(&format!(
                    "must be no more than {} characters",
                    max_length
                )));
            }
        }

        // Pattern validation
        if let Some(pattern) = &ctx.field_def.validation.pattern {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(string_value) {
                        return Err(ctx
                            .create_validation_error(&format!("must match pattern: {}", pattern)));
                    }
                }
                Err(_) => {
                    return Err(ctx.create_validation_error(&format!(
                        "has invalid regex pattern: {}",
                        pattern
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
            Value::Number(n) if n.is_u64() => n.as_u64().unwrap() as i64,
            Value::String(s) => s
                .parse::<i64>()
                .map_err(|_| ctx.create_validation_error("must be a valid integer"))?,
            _ => {
                return Err(ctx.create_validation_error("must be an integer"));
            }
        };

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
        let date_str = match ctx.value {
            Value::String(s) => s,
            _ => {
                return Err(ctx.create_validation_error("must be a date string"));
            }
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
                return Err(
                    ctx.create_validation_error(&format!("must be on or after {}", min_date))
                );
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
                    ctx.create_validation_error(&format!("must be on or before {}", max_date))
                );
            }
        }

        Ok(())
    }

    /// Validate datetime fields
    fn validate_datetime(ctx: &ValidationContext) -> Result<()> {
        let datetime_str = match ctx.value {
            Value::String(s) => s,
            _ => {
                return Err(ctx.create_validation_error("must be a datetime string"));
            }
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
                return Err(
                    ctx.create_validation_error(&format!("must be on or after {}", min_date))
                );
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
                    ctx.create_validation_error(&format!("must be on or before {}", max_date))
                );
            }
        }

        Ok(())
    }

    /// Validate UUID fields
    fn validate_uuid(ctx: &ValidationContext) -> Result<()> {
        let uuid_str = match ctx.value {
            Value::String(s) => s,
            _ => {
                return Err(ctx.create_validation_error("must be a UUID string"));
            }
        };

        Uuid::parse_str(uuid_str)
            .map_err(|_| ctx.create_validation_error("must be a valid UUID"))?;

        Ok(())
    }

    /// Validate select fields
    fn validate_select(ctx: &ValidationContext) -> Result<()> {
        let option_value = match ctx.value {
            Value::String(s) => s,
            _ => {
                return Err(ctx.create_validation_error("must be a string"));
            }
        };

        // Validate against options if present
        if let Some(options_source) = &ctx.field_def.validation.options_source {
            if let crate::entity::field::OptionsSource::Fixed { options } = options_source {
                let valid_options: Vec<String> =
                    options.iter().map(|opt| opt.value.clone()).collect();

                if !valid_options.contains(&option_value.to_string()) {
                    return Err(ctx.create_validation_error(&format!(
                        "must be one of: {}",
                        valid_options.join(", ")
                    )));
                }
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
        if let Some(options_source) = &ctx.field_def.validation.options_source {
            if let crate::entity::field::OptionsSource::Fixed { options } = options_source {
                let valid_options: Vec<String> =
                    options.iter().map(|opt| opt.value.clone()).collect();

                for value in &selected_values {
                    if !valid_options.contains(value) {
                        return Err(ctx.create_validation_error(&format!(
                            "contains invalid option '{}'. Valid options are: {}",
                            value,
                            valid_options.join(", ")
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate JSON fields
    fn validate_json(_: &ValidationContext) -> Result<()> {
        // For JSON fields, we accept any valid JSON
        // No additional validation needed as Value is already valid JSON
        Ok(())
    }
}

pub fn validate_field(field_def: &Value, value: &Value, field_name: &str) -> Result<()> {
    let field_type = field_def
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Validation(format!("Missing type for field {}", field_name)))?;

    match field_type {
        "string" => {
            if !value.is_string() {
                return Err(Error::Validation(format!(
                    "Field {} must be a string",
                    field_name
                )));
            }
            Ok(())
        }
        "number" | "integer" => {
            if !value.is_number() {
                return Err(Error::Validation(format!(
                    "Field {} must be a number",
                    field_name
                )));
            }
            Ok(())
        }
        "boolean" => {
            if !value.is_boolean() {
                return Err(Error::Validation(format!(
                    "Field {} must be a boolean",
                    field_name
                )));
            }
            Ok(())
        }
        "array" => {
            if !value.is_array() {
                return Err(Error::Validation(format!(
                    "Field {} must be an array",
                    field_name
                )));
            }
            Ok(())
        }
        "object" => {
            if !value.is_object() {
                return Err(Error::Validation(format!(
                    "Field {} must be an object",
                    field_name
                )));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn validate_entity(entity: &Value, entity_def: &EntityDefinition) -> Result<()> {
    let mut validation_errors = Vec::new();
    let entity_type = entity
        .get("entity_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Validation("Entity must have an entity_type field".to_string()))?;

    if entity_type != entity_def.entity_type {
        return Err(Error::Validation(format!(
            "Entity type '{}' does not match entity definition type '{}'",
            entity_type, entity_def.entity_type
        )));
    }

    let field_data = entity
        .get("field_data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::Validation("Entity must have a field_data object".to_string()))?;

    // Check required fields
    for field_def in &entity_def.fields {
        if field_def.required && !field_data.contains_key(&field_def.name) {
            validation_errors.push(format!("Required field '{}' is missing", field_def.name));
        }
    }

    // Validate fields that are present
    for (field_name, value) in field_data {
        if let Some(field_def) = entity_def.get_field(field_name) {
            let _ = ValidationContext::with_field_name(field_def, value, field_name);
            if let Err(e) = DynamicEntityValidator::validate_field(field_def, value) {
                validation_errors.push(e.to_string());
            }
        } else {
            // Skip system fields
            let system_fields = [
                "uuid",
                "path",
                "created_at",
                "updated_at",
                "created_by",
                "updated_by",
                "published",
                "version",
            ];
            if !system_fields.contains(&field_name.as_str()) {
                validation_errors.push(format!("Unknown field '{}'", field_name));
            }
        }
    }

    if !validation_errors.is_empty() {
        return Err(Error::Validation(format!(
            "Validation failed with the following errors: {}",
            validation_errors.join("; ")
        )));
    }

    Ok(())
}
