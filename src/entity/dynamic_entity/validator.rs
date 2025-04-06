use serde_json::Value;

use crate::error::{Error, Result};
use crate::entity::field::{FieldDefinition, FieldType};

/// Validator for dynamic entities
pub struct DynamicEntityValidator;

impl DynamicEntityValidator {
    /// Validate a field against its definition
    pub fn validate_field(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Handle null value for required fields
        if value.is_null() {
            if field_def.required {
                return Err(Error::Validation(format!("Field '{}' is required", field_def.name)));
            }
            return Ok(());
        }
        
        // Type validation
        match field_def.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                Self::validate_string(field_def, value)
            },
            FieldType::Integer => {
                Self::validate_integer(field_def, value)
            },
            FieldType::Float => {
                Self::validate_float(field_def, value)
            },
            FieldType::Boolean => {
                Self::validate_boolean(field_def, value)
            },
            FieldType::Date => {
                Self::validate_date(field_def, value)
            },
            FieldType::DateTime => {
                Self::validate_datetime(field_def, value)
            },
            FieldType::UUID => {
                Self::validate_uuid(field_def, value)
            },
            FieldType::Select => {
                Self::validate_select(field_def, value)
            },
            FieldType::MultiSelect => {
                Self::validate_multi_select(field_def, value)
            },
            _ => Ok(()),  // Other types not validated here
        }
    }
    
    /// Validate string fields
    fn validate_string(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Check type
        if !value.is_string() {
            return Err(Error::Validation(format!("Field '{}' must be a string", field_def.name)));
        }
        
        let string_value = value.as_str().unwrap();
        
        // Length validation
        if let Some(min_length) = field_def.validation.min_length {
            if string_value.len() < min_length {
                return Err(Error::Validation(format!(
                    "Field '{}' must be at least {} characters", 
                    field_def.name, 
                    min_length
                )));
            }
        }
        
        if let Some(max_length) = field_def.validation.max_length {
            if string_value.len() > max_length {
                return Err(Error::Validation(format!(
                    "Field '{}' must be no more than {} characters", 
                    field_def.name, 
                    max_length
                )));
            }
        }
        
        // Pattern validation
        if let Some(pattern) = &field_def.validation.pattern {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(string_value) {
                        return Err(Error::Validation(format!(
                            "Field '{}' must match pattern: {}", 
                            field_def.name, 
                            pattern
                        )));
                    }
                },
                Err(_) => {
                    return Err(Error::Validation(format!(
                        "Invalid regex pattern for field '{}': {}", 
                        field_def.name, 
                        pattern
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate integer fields
    fn validate_integer(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Parse integer
        let int_value = match value {
            Value::Number(n) if n.is_i64() => n.as_i64().unwrap(),
            Value::Number(n) if n.is_u64() => n.as_u64().unwrap() as i64,
            Value::String(s) => s.parse::<i64>().map_err(|_| 
                Error::Validation(format!("Field '{}' must be a valid integer", field_def.name)))?,
            _ => return Err(Error::Validation(format!("Field '{}' must be an integer", field_def.name))),
        };
        
        // Range validation
        if let Some(min_value) = &field_def.validation.min_value {
            if let Some(min) = min_value.as_i64() {
                if int_value < min {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at least {}", 
                        field_def.name, 
                        min
                    )));
                }
            }
        }
        
        if let Some(max_value) = &field_def.validation.max_value {
            if let Some(max) = max_value.as_i64() {
                if int_value > max {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be no more than {}", 
                        field_def.name, 
                        max
                    )));
                }
            }
        }
        
        // Positive only validation
        if let Some(true) = field_def.validation.positive_only {
            if int_value < 0 {
                return Err(Error::Validation(format!(
                    "Field '{}' must be a positive number", 
                    field_def.name
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate float fields
    fn validate_float(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Parse float
        let float_value = match value {
            Value::Number(n) => n.as_f64().unwrap(),
            Value::String(s) => s.parse::<f64>().map_err(|_| 
                Error::Validation(format!("Field '{}' must be a valid number", field_def.name)))?,
            _ => return Err(Error::Validation(format!("Field '{}' must be a number", field_def.name))),
        };
        
        // Range validation
        if let Some(min_value) = &field_def.validation.min_value {
            if let Some(min) = min_value.as_f64() {
                if float_value < min {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at least {}", 
                        field_def.name, 
                        min
                    )));
                }
            }
        }
        
        if let Some(max_value) = &field_def.validation.max_value {
            if let Some(max) = max_value.as_f64() {
                if float_value > max {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be no more than {}", 
                        field_def.name, 
                        max
                    )));
                }
            }
        }
        
        // Positive only validation
        if let Some(true) = field_def.validation.positive_only {
            if float_value < 0.0 {
                return Err(Error::Validation(format!(
                    "Field '{}' must be a positive number", 
                    field_def.name
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate boolean fields
    fn validate_boolean(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        match value {
            Value::Bool(_) => Ok(()),
            Value::String(s) => {
                match s.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "false" | "no" | "0" => Ok(()),
                    _ => Err(Error::Validation(format!(
                        "Field '{}' must be a boolean value", 
                        field_def.name
                    )))
                }
            },
            Value::Number(n) => {
                if n.as_i64() == Some(0) || n.as_i64() == Some(1) {
                    Ok(())
                } else {
                    Err(Error::Validation(format!(
                        "Field '{}' must be a boolean value (0 or 1)", 
                        field_def.name
                    )))
                }
            },
            _ => Err(Error::Validation(format!("Field '{}' must be a boolean", field_def.name))),
        }
    }
    
    /// Validate date fields
    fn validate_date(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Parse date
        let date_str = match value {
            Value::String(s) => s,
            _ => return Err(Error::Validation(format!("Field '{}' must be a date string", field_def.name))),
        };
        
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| Error::Validation(format!(
                "Field '{}' must be a valid date in YYYY-MM-DD format", 
                field_def.name
            )))?;
        
        // Date range validation
        let now = Utc::now().date_naive();
        
        if let Some(min_date_str) = &field_def.validation.min_date {
            let min_date = if min_date_str == "now" {
                now
            } else {
                NaiveDate::parse_from_str(min_date_str, "%Y-%m-%d")
                    .map_err(|_| Error::Validation(format!(
                        "Invalid min_date format for field '{}': {}", 
                        field_def.name, 
                        min_date_str
                    )))?
            };
            
            if date < min_date {
                return Err(Error::Validation(format!(
                    "Field '{}' must be on or after {}", 
                    field_def.name, 
                    min_date
                )));
            }
        }
        
        if let Some(max_date_str) = &field_def.validation.max_date {
            let max_date = if max_date_str == "now" {
                now
            } else {
                NaiveDate::parse_from_str(max_date_str, "%Y-%m-%d")
                    .map_err(|_| Error::Validation(format!(
                        "Invalid max_date format for field '{}': {}", 
                        field_def.name, 
                        max_date_str
                    )))?
            };
            
            if date > max_date {
                return Err(Error::Validation(format!(
                    "Field '{}' must be on or before {}", 
                    field_def.name, 
                    max_date
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate datetime fields
    fn validate_datetime(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        // Parse datetime
        let datetime_str = match value {
            Value::String(s) => s,
            _ => return Err(Error::Validation(format!(
                "Field '{}' must be a datetime string", 
                field_def.name
            ))),
        };
        
        let datetime = DateTime::parse_from_rfc3339(datetime_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| Error::Validation(format!(
                "Field '{}' must be a valid datetime in RFC3339 format", 
                field_def.name
            )))?;
        
        // Datetime range validation
        let now = Utc::now();
        
        if let Some(min_date_str) = &field_def.validation.min_date {
            let min_date = if min_date_str == "now" {
                now
            } else {
                DateTime::parse_from_rfc3339(min_date_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| Error::Validation(format!(
                        "Invalid min_date format for field '{}': {}", 
                        field_def.name, 
                        min_date_str
                    )))?
            };
            
            if datetime < min_date {
                return Err(Error::Validation(format!(
                    "Field '{}' must be on or after {}", 
                    field_def.name, 
                    min_date
                )));
            }
        }
        
        if let Some(max_date_str) = &field_def.validation.max_date {
            let max_date = if max_date_str == "now" {
                now
            } else {
                DateTime::parse_from_rfc3339(max_date_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| Error::Validation(format!(
                        "Invalid max_date format for field '{}': {}", 
                        field_def.name, 
                        max_date_str
                    )))?
            };
            
            if datetime > max_date {
                return Err(Error::Validation(format!(
                    "Field '{}' must be on or before {}", 
                    field_def.name, 
                    max_date
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate UUID fields
    fn validate_uuid(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        let uuid_str = match value {
            Value::String(s) => s,
            _ => return Err(Error::Validation(format!("Field '{}' must be a UUID string", field_def.name))),
        };
        
        Uuid::parse_str(uuid_str)
            .map_err(|_| Error::Validation(format!(
                "Field '{}' must be a valid UUID", 
                field_def.name
            )))?;
        
        Ok(())
    }
    
    /// Validate select fields
    fn validate_select(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        let option_value = match value {
            Value::String(s) => s,
            _ => return Err(Error::Validation(format!("Field '{}' must be a string", field_def.name))),
        };
        
        // Validate against options if present
        if let Some(options_source) = &field_def.validation.options_source {
            match options_source {
                crate::entity::field::OptionsSource::Fixed { options } => {
                    let valid_options: Vec<String> = options.iter()
                        .map(|opt| opt.value.clone())
                        .collect();
                    
                    if !valid_options.contains(&option_value.to_string()) {
                        return Err(Error::Validation(format!(
                            "Field '{}' must be one of: {}", 
                            field_def.name, 
                            valid_options.join(", ")
                        )));
                    }
                },
                _ => {}, // Can't validate Enum or Query sources here
            }
        }
        
        Ok(())
    }
    
    /// Validate multi-select fields
    fn validate_multi_select(field_def: &FieldDefinition, value: &Value) -> Result<()> {
        let selected_values = match value {
            Value::Array(arr) => arr.iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.clone()),
                    _ => Err(Error::Validation(format!(
                        "Field '{}' must contain only strings", 
                        field_def.name
                    ))),
                })
                .collect::<Result<Vec<String>>>()?,
            Value::String(s) => vec![s.clone()],
            _ => return Err(Error::Validation(format!(
                "Field '{}' must be an array of strings", 
                field_def.name
            ))),
        };
        
        // Validate against options if present
        if let Some(options_source) = &field_def.validation.options_source {
            match options_source {
                crate::entity::field::OptionsSource::Fixed { options } => {
                    let valid_options: Vec<String> = options.iter()
                        .map(|opt| opt.value.clone())
                        .collect();
                    
                    for value in &selected_values {
                        if !valid_options.contains(value) {
                            return Err(Error::Validation(format!(
                                "Field '{}' contains invalid option '{}'. Valid options are: {}", 
                                field_def.name, 
                                value,
                                valid_options.join(", ")
                            )));
                        }
                    }
                },
                _ => {}, // Can't validate Enum or Query sources here
            }
        }
        
        Ok(())
    }
} 