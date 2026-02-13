use regex::Regex;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::field::definition::FieldDefinition;
use crate::field::types::FieldType;

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
            FieldType::String | FieldType::Text | FieldType::Wysiwyg | FieldType::Password => {
                self.validate_string_value(value)?;
            }
            FieldType::Integer => {
                self.validate_integer_value(value)?;
            }
            FieldType::Float => {
                self.validate_float_value(value)?;
            }
            FieldType::Boolean => {
                self.validate_boolean_value(value)?;
            }
            FieldType::DateTime | FieldType::Date => {
                self.validate_date_value(value)?;
            }
            FieldType::Uuid => {
                self.validate_uuid_value(value)?;
            }
            FieldType::Select => {
                self.validate_select_value(value)?;
            }
            FieldType::MultiSelect => {
                self.validate_multiselect_value(value)?;
            }
            FieldType::Array => {
                self.validate_array_value(value)?;
            }
            FieldType::Object => {
                self.validate_object_value(value)?;
            }
            // Json accepts any valid JSON value (objects, arrays, strings, numbers, booleans)
            // No additional validation needed since serde_json already ensures valid JSON.
            // Other types (ManyToOne, ManyToMany, Image, File) also skip validation for now.
            FieldType::Json
            | FieldType::ManyToOne
            | FieldType::ManyToMany
            | FieldType::Image
            | FieldType::File => {}
        }

        Ok(())
    }

    /// Validate a string value
    fn validate_string_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a string",
                self.name
            )));
        }

        let s = value.as_str().unwrap();

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
    fn validate_integer_value(&self, value: &Value) -> Result<()> {
        if !value.is_i64() && !value.is_u64() {
            if value.is_string() {
                // Try to parse as integer
                let s = value.as_str().unwrap();
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
        let n = if value.is_i64() {
            value.as_i64().unwrap() as f64
        } else if value.is_u64() {
            value.as_u64().unwrap() as f64
        } else {
            // Must be a string at this point
            value.as_str().unwrap().parse::<i64>().unwrap() as f64
        };

        self.validate_numeric_constraints(n)
    }

    /// Validate a float value
    fn validate_float_value(&self, value: &Value) -> Result<()> {
        if !value.is_f64() && !value.is_i64() && !value.is_u64() {
            if value.is_string() {
                // Try to parse as float
                let s = value.as_str().unwrap();
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
        let n = if value.is_f64() {
            value.as_f64().unwrap()
        } else if value.is_i64() {
            value.as_i64().unwrap() as f64
        } else if value.is_u64() {
            value.as_u64().unwrap() as f64
        } else {
            // Must be a string at this point
            value.as_str().unwrap().parse::<f64>().unwrap()
        };

        self.validate_numeric_constraints(n)
    }

    /// Validate numeric constraints (min, max, `positive_only`)
    fn validate_numeric_constraints(&self, n: f64) -> Result<()> {
        // Check min value
        if let Some(min_value) = &self.validation.min_value {
            if let Some(min) = min_value.as_f64() {
                if n < min {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at least {}",
                        self.name, min
                    )));
                }
            }
        }

        // Check max value
        if let Some(max_value) = &self.validation.max_value {
            if let Some(max) = max_value.as_f64() {
                if n > max {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at most {}",
                        self.name, max
                    )));
                }
            }
        }

        // Check positive only
        if let Some(positive_only) = self.validation.positive_only {
            if positive_only && n < 0.0 {
                return Err(Error::Validation(format!(
                    "Field '{}' must be positive",
                    self.name
                )));
            }
        }

        Ok(())
    }

    /// Validate a boolean value
    ///
    /// Only accepts actual JSON booleans (`true` / `false`).
    /// Strings like `"true"` or numbers like `1` are rejected — callers must
    /// send the correct JSON type.
    fn validate_boolean_value(&self, value: &Value) -> Result<()> {
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
    fn validate_date_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a date string",
                self.name
            )));
        }

        let date_str = value.as_str().unwrap();

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
    fn validate_uuid_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a UUID string",
                self.name
            )));
        }

        let uuid_str = value.as_str().unwrap();

        // Try to parse the UUID
        if Uuid::parse_str(uuid_str).is_err() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a valid UUID",
                self.name
            )));
        }

        Ok(())
    }

    /// Validate a select value
    fn validate_select_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a string",
                self.name
            )));
        }

        let selected = value.as_str().unwrap();

        // Check if selected value is in options
        if let Some(crate::field::options::OptionsSource::Fixed { options }) =
            &self.validation.options_source
        {
            let valid_options: Vec<&String> = options.iter().map(|opt| &opt.value).collect();

            if !valid_options.contains(&&selected.to_string()) {
                return Err(Error::Validation(format!(
                    "Field '{}' value must be one of {:?}",
                    self.name, valid_options
                )));
            }
        }

        Ok(())
    }

    /// Validate a multiselect value
    fn validate_multiselect_value(&self, value: &Value) -> Result<()> {
        if !value.is_array() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an array",
                self.name
            )));
        }

        let selected = value.as_array().unwrap();

        // Check if all selected values are strings
        for item in selected {
            if !item.is_string() {
                return Err(Error::Validation(format!(
                    "Field '{}' must contain only string values",
                    self.name
                )));
            }
        }

        // Check if selected values are in options
        if let Some(crate::field::options::OptionsSource::Fixed { options }) =
            &self.validation.options_source
        {
            let valid_options: Vec<&String> = options.iter().map(|opt| &opt.value).collect();

            for item in selected {
                let item_str = item.as_str().unwrap();
                if !valid_options.contains(&&item_str.to_string()) {
                    return Err(Error::Validation(format!(
                        "Field '{}' values must be one of {:?}",
                        self.name, valid_options
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate an array value
    fn validate_array_value(&self, value: &Value) -> Result<()> {
        if !value.is_array() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an array",
                self.name
            )));
        }

        Ok(())
    }

    /// Validate an object value
    fn validate_object_value(&self, value: &Value) -> Result<()> {
        if !value.is_object() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an object",
                self.name
            )));
        }

        Ok(())
    }

    /// Validate this field definition for common issues like invalid constraints
    ///
    /// # Errors
    /// Returns an error if validation fails
    #[allow(clippy::too_many_lines)] // Complex validation logic requires many lines
    pub fn validate(&self) -> Result<()> {
        // Check if the field has a valid name
        if self.name.is_empty() {
            return Err(Error::Validation("Field name cannot be empty".to_string()));
        }

        // Check for valid display name
        if self.display_name.is_empty() {
            return Err(Error::Validation(
                "Field display name cannot be empty".to_string(),
            ));
        }

        // Check for reserved SQL keywords
        let reserved_keywords = [
            "all",
            "analyse",
            "analyze",
            "and",
            "any",
            "array",
            "as",
            "asc",
            "asymmetric",
            "authorization",
            "binary",
            "both",
            "case",
            "cast",
            "check",
            "collate",
            "column",
            "constraint",
            "create",
            "cross",
            "current_date",
            "current_role",
            "current_time",
            "current_timestamp",
            "current_user",
            "default",
            "deferrable",
            "desc",
            "distinct",
            "do",
            "else",
            "end",
            "except",
            "false",
            "for",
            "foreign",
            "freeze",
            "from",
            "full",
            "grant",
            "group",
            "having",
            "in",
            "initially",
            "inner",
            "intersect",
            "into",
            "is",
            "isnull",
            "join",
            "leading",
            "left",
            "like",
            "limit",
            "localtime",
            "localtimestamp",
            "natural",
            "not",
            "notnull",
            "null",
            "offset",
            "on",
            "only",
            "or",
            "order",
            "outer",
            "overlaps",
            "placing",
            "primary",
            "references",
            "right",
            "select",
            "session_user",
            "similar",
            "some",
            "symmetric",
            "table",
            "then",
            "to",
            "trailing",
            "true",
            "union",
            "unique",
            "user",
            "using",
            "when",
            "where",
            "with",
        ];

        if reserved_keywords.contains(&self.name.to_lowercase().as_str()) {
            return Err(Error::Validation(format!(
                "Field name '{}' is a reserved SQL keyword and cannot be used",
                self.name
            )));
        }

        // Validate constraints based on field type
        for (constraint_type, constraint_value) in &self.constraints {
            self.handle_constraint(constraint_type, constraint_value)?;
        }

        Ok(())
    }
}
