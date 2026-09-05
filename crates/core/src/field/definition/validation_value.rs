use regex::Regex;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::field::definition::FieldDefinition;

impl FieldDefinition {
    /// Validate a field value against this definition
    ///
    /// # Panics
    /// May panic if value is not a string when checking for empty strings
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub fn validate_value(&self, value: &Value) -> Result<()> {
        // Check if required
        if self.required && value.is_null() {
            return Err(Error::Validation(format!(
                "Field '{}' is required",
                self.name
            )));
        }

        // Skip validation for null values if not required
        if value.is_null() {
            return Ok(());
        }

        // Perform type-specific validations
        match self.field_type {
            crate::field::types::FieldType::String
            | crate::field::types::FieldType::Text
            | crate::field::types::FieldType::Wysiwyg
            | crate::field::types::FieldType::Password => {
                self.validate_string_value(value)?;
            }
            crate::field::types::FieldType::Integer => {
                self.validate_integer_value(value)?;
            }
            crate::field::types::FieldType::Float => {
                self.validate_float_value(value)?;
            }
            crate::field::types::FieldType::Boolean => {
                self.validate_boolean_value(value)?;
            }
            crate::field::types::FieldType::DateTime | crate::field::types::FieldType::Date => {
                self.validate_date_value(value)?;
            }
            crate::field::types::FieldType::Uuid => {
                self.validate_uuid_value(value)?;
            }
            crate::field::types::FieldType::Select => {
                self.validate_select_value(value)?;
            }
            crate::field::types::FieldType::MultiSelect => {
                self.validate_multiselect_value(value)?;
            }
            crate::field::types::FieldType::Array => {
                self.validate_array_value(value)?;
            }
            crate::field::types::FieldType::Object => {
                self.validate_object_value(value)?;
            }
            // Json accepts any valid JSON value (objects, arrays, strings, numbers, booleans)
            // No additional validation needed since serde_json already ensures valid JSON.
            // Other types (ManyToOne, ManyToMany, Image, File) also skip validation for now.
            crate::field::types::FieldType::Json
            | crate::field::types::FieldType::ManyToOne
            | crate::field::types::FieldType::ManyToMany
            | crate::field::types::FieldType::Image
            | crate::field::types::FieldType::File => {}
        }

        Ok(())
    }

    /// Validate a string value
    pub(super) fn validate_string_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a string",
                self.name
            )));
        }

        // as_str() is infallible here: is_string() guard returned Err above
        let s = value.as_str().unwrap_or_default();

        // Check min length
        if let Some(min_length) = self.validation.min_length {
            if s.len() < min_length {
                return Err(Error::Validation(format!(
                    "Field '{}' must be at least {} characters",
                    self.name, min_length
                )));
            }
        }

        // Check max length
        if let Some(max_length) = self.validation.max_length {
            if s.len() > max_length {
                return Err(Error::Validation(format!(
                    "Field '{}' must be at most {} characters",
                    self.name, max_length
                )));
            }
        }

        // Check pattern
        if let Some(pattern) = &self.validation.pattern {
            // Skip pattern validation for empty strings if field is not required
            if !self.required && s.is_empty() {
                // Allow empty strings for optional fields
            } else {
                match Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(s) {
                            return Err(Error::Validation(format!(
                                "Field '{}' does not match pattern",
                                self.name
                            )));
                        }
                    }
                    Err(_) => {
                        return Err(Error::Validation(format!(
                            "Invalid pattern for field '{}'",
                            self.name
                        )))
                    }
                }
            }
        }

        // Check enum options if present
        if let Some(crate::field::options::OptionsSource::Fixed { options }) =
            &self.validation.options_source
        {
            let valid_options: Vec<&String> = options.iter().map(|opt| &opt.value).collect();

            if !valid_options.contains(&&s.to_string()) {
                return Err(Error::Validation(format!(
                    "Field '{}' value must be one of {:?}",
                    self.name, valid_options
                )));
            }
        }

        Ok(())
    }

    /// Validate an integer value
    pub(super) fn validate_integer_value(&self, value: &Value) -> Result<()> {
        if !value.is_i64() && !value.is_u64() {
            if value.is_string() {
                // Try to parse as integer
                // unwrap_or_default is safe: is_string() guard is true here
                let s = value.as_str().unwrap_or_default();
                if s.parse::<i64>().is_err() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be an integer",
                        self.name
                    )));
                }
            } else {
                return Err(Error::Validation(format!(
                    "Field '{}' must be an integer",
                    self.name
                )));
            }
        }

        #[allow(clippy::cast_precision_loss)] // i64/u64 to f64 conversion for validation
        #[allow(clippy::unwrap_used)]
        // each branch is guarded by is_i64()/is_u64()/is_string() — infallible
        let n = if value.is_i64() {
            value.as_i64().unwrap() as f64
        } else if value.is_u64() {
            value.as_u64().unwrap() as f64
        } else {
            // Must be a string at this point (earlier branch returned Err otherwise)
            value.as_str().unwrap().parse::<i64>().unwrap() as f64
        };

        self.validate_numeric_constraints(n)
    }

    /// Validate a float value
    pub(super) fn validate_float_value(&self, value: &Value) -> Result<()> {
        if !value.is_f64() && !value.is_i64() && !value.is_u64() {
            if value.is_string() {
                // Try to parse as float
                // unwrap_or_default is safe: is_string() guard is true here
                let s = value.as_str().unwrap_or_default();
                if s.parse::<f64>().is_err() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a number",
                        self.name
                    )));
                }
            } else {
                return Err(Error::Validation(format!(
                    "Field '{}' must be a number",
                    self.name
                )));
            }
        }

        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::unwrap_used)]
        // each branch guarded by is_f64()/is_i64()/is_u64()/is_string() — infallible
        let n = if value.is_f64() {
            value.as_f64().unwrap()
        } else if value.is_i64() {
            value.as_i64().unwrap() as f64
        } else if value.is_u64() {
            value.as_u64().unwrap() as f64
        } else {
            // Must be a string at this point (earlier branch returned Err otherwise)
            value.as_str().unwrap().parse::<f64>().unwrap()
        };

        self.validate_numeric_constraints(n)
    }

    /// Validate a boolean value
    ///
    /// Only accepts actual JSON booleans (`true` / `false`).
    /// Strings like `"true"` or numbers like `1` are rejected — callers must
    /// send the correct JSON type.
    pub(super) fn validate_boolean_value(&self, value: &Value) -> Result<()> {
        if !value.is_boolean() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a boolean (true/false), got {}",
                self.name,
                match value {
                    Value::String(s) => format!("string \"{s}\""),
                    Value::Number(n) => format!("number {n}"),
                    _ => format!("{value}"),
                }
            )));
        }

        Ok(())
    }

    /// Validate a date value
    pub(super) fn validate_date_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a date string",
                self.name
            )));
        }

        // unwrap_or_default is safe: is_string() guard returned Err above
        let date_str = value.as_str().unwrap_or_default();

        // Try to parse the date
        if time::OffsetDateTime::parse(date_str, &Rfc3339).is_err() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a valid date in RFC3339 format",
                self.name
            )));
        }

        // Check min date
        if let Some(min_date) = &self.validation.min_date {
            if let Ok(min) = time::OffsetDateTime::parse(min_date, &Rfc3339) {
                if let Ok(date) = time::OffsetDateTime::parse(date_str, &Rfc3339) {
                    if date < min {
                        return Err(Error::Validation(format!(
                            "Field '{}' must be after {}",
                            self.name, min_date
                        )));
                    }
                }
            }
        }

        // Check max date
        if let Some(max_date) = &self.validation.max_date {
            if let Ok(max) = time::OffsetDateTime::parse(max_date, &Rfc3339) {
                if let Ok(date) = time::OffsetDateTime::parse(date_str, &Rfc3339) {
                    if date > max {
                        return Err(Error::Validation(format!(
                            "Field '{}' must be before {}",
                            self.name, max_date
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a UUID value
    pub(super) fn validate_uuid_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a UUID string",
                self.name
            )));
        }

        // unwrap_or_default is safe: is_string() guard returned Err above
        let uuid_str = value.as_str().unwrap_or_default();

        // Try to parse the UUID
        if Uuid::parse_str(uuid_str).is_err() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a valid UUID",
                self.name
            )));
        }

        Ok(())
    }
}
