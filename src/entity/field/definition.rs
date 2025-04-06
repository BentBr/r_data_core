use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

use super::types::FieldType;
use super::options::FieldValidation;
use super::ui::UiSettings;
use crate::error::{Error, Result};

/// Definition of a field in a class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name (must be unique within class)
    pub name: String,
    
    /// User-friendly display name
    pub display_name: String,
    
    /// Field data type
    pub field_type: FieldType,
    
    /// Field description for admin UI
    pub description: Option<String>,
    
    /// Whether the field is required
    pub required: bool,
    
    /// Whether the field is indexed for faster searches
    pub indexed: bool,
    
    /// Whether the field can be used in API filtering
    pub filterable: bool,
    
    /// Default value for the field as JSON
    pub default_value: Option<Value>,
    
    /// Field validation/constraints
    #[serde(default)]
    pub validation: FieldValidation,
    
    /// UI settings for the field
    #[serde(default)]
    pub ui_settings: UiSettings,
    
    /// Extra field constraints or validation rules
    pub constraints: HashMap<String, Value>,
}

impl FieldDefinition {
    /// Create a new field definition with default values
    pub fn new(name: String, display_name: String, field_type: FieldType) -> Self {
        Self {
            name,
            display_name,
            field_type,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }
    }
    
    /// Validate a value against this field definition
    pub fn validate_value(&self, value: &Value) -> Result<()> {
        // If the field is required, it can't be null
        if self.required && (value.is_null() || (value.is_string() && value.as_str().unwrap_or("").is_empty())) {
            return Err(Error::Validation(format!("Field '{}' is required but value is null or empty", self.name)));
        }

        // Check validation rules based on field type
        match self.field_type {
            FieldType::String => {
                if let Some(min_length) = self.validation.min_length {
                    if let Some(s) = value.as_str() {
                        if s.len() < min_length as usize {
                            return Err(Error::Validation(format!("String is too short, minimum length is {}", min_length)));
                        }
                    }
                }
                if let Some(max_length) = self.validation.max_length {
                    if let Some(s) = value.as_str() {
                        if s.len() > max_length as usize {
                            return Err(Error::Validation(format!("String is too long, maximum length is {}", max_length)));
                        }
                    }
                }
                if let Some(pattern) = &self.validation.pattern {
                    if let Some(s) = value.as_str() {
                        let re = regex::Regex::new(pattern).map_err(|e| Error::Validation(format!("Invalid regex pattern: {}", e)))?;
                        if !re.is_match(s) {
                            return Err(Error::Validation(format!("String does not match pattern: {}", pattern)));
                        }
                    }
                }
            },
            FieldType::Integer => {
                if let Some(value_int) = value.as_i64() {
                    // Validate integer range
                    if let Some(min_value) = self.validation.min_value.as_ref().and_then(|v| v.as_i64()) {
                        if value_int < min_value {
                            return Err(Error::Validation(format!("Field '{}' must be at least {}", self.name, min_value)));
                        }
                    }

                    if let Some(max_value) = self.validation.max_value.as_ref().and_then(|v| v.as_i64()) {
                        if value_int > max_value {
                            return Err(Error::Validation(format!("Field '{}' must be at most {}", self.name, max_value)));
                        }
                    }
                }
            },
            FieldType::Float => {
                if let Some(value_float) = value.as_f64() {
                    // Validate float range
                    if let Some(min_value) = self.validation.min_value.as_ref().and_then(|v| v.as_f64()) {
                        if value_float < min_value {
                            return Err(Error::Validation(format!("Field '{}' must be at least {}", self.name, min_value)));
                        }
                    }

                    if let Some(max_value) = self.validation.max_value.as_ref().and_then(|v| v.as_f64()) {
                        if value_float > max_value {
                            return Err(Error::Validation(format!("Field '{}' must be at most {}", self.name, max_value)));
                        }
                    }
                }
            },
            _ => {} // Other types don't need validation yet
        }

        Ok(())
    }
} 