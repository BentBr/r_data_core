#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use regex::Regex;
use serde_json::Value;
use time::{macros::format_description, Date, OffsetDateTime};
use uuid::Uuid;

use crate::error::Result;
use crate::field::{FieldDefinition, FieldType};

use super::context::ValidationContext;

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
            FieldType::String | FieldType::Text | FieldType::Wysiwyg | FieldType::Password => {
                Self::validate_string(&ctx)
            }
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

        let Some(string_value) = ctx.value.as_str() else {
            return Err(ctx.create_validation_error("must be a string"));
        };

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
            #[allow(clippy::unwrap_used)] // guarded by is_i64() / is_u64() — infallible
            Value::Number(n) if n.is_i64() => n.as_i64().unwrap(),
            #[allow(clippy::unwrap_used)] // guarded by is_i64() / is_u64() — infallible
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
            #[allow(clippy::unwrap_used)]
            // Value::Number always yields Some from as_f64() — infallible
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
